use std::collections::HashMap;
use std::ffi::{OsString, c_void};
use std::io::Read;
use std::os::windows::{ffi::OsStringExt, process::CommandExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::System::Ole::*;
use windows::Win32::System::Threading::CREATE_NO_WINDOW;
use windows::Win32::UI::Shell::*;
use windows::core::*;

pub const COMMAND_CLSID: GUID = GUID::from_u128(0x4987c7b0_bfc5_41b0_94c8_7d06e3f1471c);

static LIVE_OBJECTS: AtomicUsize = AtomicUsize::new(0);
static SERVER_LOCKS: AtomicUsize = AtomicUsize::new(0);

struct LiveGuard;
impl LiveGuard {
    fn take() -> Self {
        LIVE_OBJECTS.fetch_add(1, Ordering::SeqCst);
        Self
    }
}
impl Drop for LiveGuard {
    fn drop(&mut self) { LIVE_OBJECTS.fetch_sub(1, Ordering::SeqCst); }
}

fn fail() -> Error { Error::from_hresult(E_FAIL) }

fn alloc_shell_string(value: &str) -> Result<PWSTR> {
    let h = HSTRING::from(value);
    unsafe { SHStrDupW(&h) }
}

fn module_dir() -> Result<PathBuf> {
    let mut module = HMODULE::default();
    let mut buf = vec![0u16; 32768];
    unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(DllGetClassObject as *const () as *const u16),
            &mut module,
        )?;
        let n = GetModuleFileNameW(Some(module), &mut buf) as usize;
        if n == 0 || n >= buf.len() { return Err(fail()); }
        let dll = PathBuf::from(OsString::from_wide(&buf[..n]));
        dll.parent().map(Path::to_path_buf).ok_or_else(fail)
    }
}

fn paths_from(items: Ref<IShellItemArray>) -> Result<Vec<PathBuf>> {
    let array = items.ok()?;
    let mut result = Vec::new();
    unsafe {
        let count = array.GetCount()?;
        if count == 0 || count > 256 { return Err(fail()); }
        for i in 0..count {
            let item = array.GetItemAt(i)?;
            let raw = item.GetDisplayName(SIGDN_FILESYSPATH)?;
            let p = PathBuf::from(OsString::from_wide(raw.as_wide()));
            CoTaskMemFree(Some(raw.0.cast()));
            if !p.is_absolute() || !p.is_file() { return Err(fail()); }
            result.push(p);
        }
    }
    Ok(result)
}

const CACHE_AGE: Duration = Duration::from_secs(60);
const PROBE_TIMEOUT: Duration = Duration::from_millis(900);
type TargetCache = HashMap<OsString, (Instant, Vec<String>)>;
static MENU_CACHE: OnceLock<Mutex<TargetCache>> = OnceLock::new();

fn targets_for(path: &Path) -> Vec<String> {
    let Some(ext) = path.extension().map(|x| x.to_ascii_lowercase()) else { return vec![]; };
    let cache = MENU_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(c) = cache.lock() {
        if let Some((t, values)) = c.get(&ext) {
            if t.elapsed() < CACHE_AGE { return values.clone(); }
        }
    }
    let values = probe_targets(path).unwrap_or_default();
    if let Ok(mut c) = cache.lock() {
        if c.len() > 128 { c.clear(); }
        c.insert(ext, (Instant::now(), values.clone()));
    }
    values
}

fn probe_targets(path: &Path) -> Option<Vec<String>> {
    let exe = module_dir().ok()?.join("m5convert.exe");
    let mut child = Command::new(exe)
        .arg("targets")
        .arg(path)
        .arg("--menu")
        .creation_flags(CREATE_NO_WINDOW.0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.take(4097).read_to_end(&mut bytes).ok()?;
        (bytes.len() <= 4096).then_some(bytes)
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed() < PROBE_TIMEOUT => std::thread::sleep(Duration::from_millis(8)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    }?;
    let data = reader.join().ok()??;
    if !status.success() { return None; }
    let text = String::from_utf8(data).ok()?;
    let values: Vec<String> = text.split_whitespace().map(str::to_owned).collect();
    values.iter().all(|s| s.len() <= 16 && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()))
        .then_some(values)
}

fn common_targets(files: &[PathBuf]) -> Vec<String> {
    let mut iter = files.iter();
    let Some(first) = iter.next() else { return vec![]; };
    let mut common = targets_for(first);
    for file in iter {
        let current = targets_for(file);
        common.retain(|x| current.contains(x));
        if common.is_empty() { break; }
    }
    common
}

fn launch_conversion(files: &[PathBuf], target: &str) -> Result<()> {
    let mut cmd = Command::new(module_dir()?.join("m5convert.exe"));
    cmd.arg("convert")
        .arg("--notify")
        .arg("--to").arg(target)
        .arg("--");
    for p in files { cmd.arg(p); }
    cmd.creation_flags(CREATE_NO_WINDOW.0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| fail())?;
    Ok(())
}

#[implement(IExplorerCommand, IObjectWithSite)]
struct MenuCommand {
    _guard: LiveGuard,
    format: Option<String>,
    site: Mutex<Option<IUnknown>>,
    saved_selection: Mutex<Vec<PathBuf>>,
}

impl MenuCommand {
    fn root() -> Self {
        Self {
            _guard: LiveGuard::take(),
            format: None,
            site: Mutex::new(None),
            saved_selection: Mutex::new(Vec::new()),
        }
    }
    fn child(format: String, selection: Vec<PathBuf>) -> Self {
        Self {
            _guard: LiveGuard::take(),
            format: Some(format),
            site: Mutex::new(None),
            saved_selection: Mutex::new(selection),
        }
    }
}

impl IObjectWithSite_Impl for MenuCommand_Impl {
    fn SetSite(&self, site: Ref<IUnknown>) -> Result<()> {
        *self.site.lock().map_err(|_| fail())? = site.cloned();
        Ok(())
    }

    fn GetSite(&self, iid: *const GUID, object: *mut *mut c_void) -> Result<()> {
        if iid.is_null() || object.is_null() { return Err(Error::from_hresult(E_POINTER)); }
        unsafe { *object = std::ptr::null_mut(); }
        let site = self.site.lock().map_err(|_| fail())?;
        unsafe { site.as_ref().ok_or_else(fail)?.query(iid, object).ok() }
    }
}

impl MenuCommand_Impl {
    fn selection(&self) -> Result<Vec<PathBuf>> {
        let saved = self.saved_selection.lock().map_err(|_| fail())?.clone();
        if !saved.is_empty() { return Ok(saved); }
        let site = self.site.lock().map_err(|_| fail())?.clone().ok_or_else(fail)?;
        let provider: IServiceProvider = site.cast()?;
        unsafe {
            let browser: IShellBrowser = provider.QueryService(&SID_STopLevelBrowser)?;
            let view = browser.QueryActiveShellView()?;
            let selected: IShellItemArray = view.GetItemObject(SVGIO_SELECTION)?;
            let files = paths_from((&selected).into())?;
            *self.saved_selection.lock().map_err(|_| fail())? = files.clone();
            Ok(files)
        }
    }
}

impl IExplorerCommand_Impl for MenuCommand_Impl {
    fn GetTitle(&self, items: Ref<IShellItemArray>) -> Result<PWSTR> {
        if self.format.is_none() {
            if let Ok(v) = paths_from(items) {
                *self.saved_selection.lock().map_err(|_| fail())? = v;
            }
        }
        match &self.format {
            Some(id) => alloc_shell_string(&id.to_uppercase()),
            None => alloc_shell_string("Convert with M5Convert"),
        }
    }

    fn GetIcon(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        Err(Error::from_hresult(E_NOTIMPL))
    }

    fn GetToolTip(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
        alloc_shell_string("Convert locally on this PC")
    }

    fn GetCanonicalName(&self) -> Result<GUID> {
        let mut id = COMMAND_CLSID;
        if let Some(fmt) = &self.format {
            let hash = fmt.bytes().fold(0x811c9dc5u32, |h,b| (h ^ b as u32).wrapping_mul(0x01000193));
            id.data1 ^= hash;
        }
        Ok(id)
    }

    fn GetState(&self, items: Ref<IShellItemArray>, slow: BOOL) -> Result<u32> {
        if self.format.is_some() { return Ok(ECS_ENABLED.0 as u32); }
        if !slow.as_bool() { return Err(Error::from_hresult(HRESULT(0x8000000A_u32 as i32))); }
        let Ok(files) = paths_from(items) else { return Ok(ECS_HIDDEN.0 as u32); };
        let shown = !common_targets(&files).is_empty();
        *self.saved_selection.lock().map_err(|_| fail())? = files;
        Ok(if shown { ECS_ENABLED.0 as u32 } else { ECS_HIDDEN.0 as u32 })
    }

    fn Invoke(&self, items: Ref<IShellItemArray>, _ctx: Ref<IBindCtx>) -> Result<()> {
        let target = self.format.as_deref().ok_or_else(fail)?;
        let files = if items.is_some() { paths_from(items)? } else { self.selection()? };
        if files.is_empty() { return Err(fail()); }
        launch_conversion(&files, target)
    }

    fn GetFlags(&self) -> Result<u32> {
        Ok(if self.format.is_none() { ECF_HASSUBCOMMANDS.0 as u32 } else { 0 })
    }

    fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
        let files = self.selection()?;
        let commands = common_targets(&files)
            .into_iter()
            .map(|fmt| -> IExplorerCommand { MenuCommand::child(fmt, files.clone()).into() })
            .collect();
        Ok(CommandEnum {
            _guard: LiveGuard::take(),
            commands,
            index: Mutex::new(0),
        }.into())
    }
}

#[implement(IEnumExplorerCommand)]
struct CommandEnum {
    _guard: LiveGuard,
    commands: Vec<IExplorerCommand>,
    index: Mutex<usize>,
}

impl IEnumExplorerCommand_Impl for CommandEnum_Impl {
    fn Next(&self, requested: u32, out: *mut Option<IExplorerCommand>, fetched: *mut u32) -> HRESULT {
        if out.is_null() || (fetched.is_null() && requested != 1) { return E_POINTER; }
        let Ok(mut index) = self.index.lock() else { return E_FAIL; };
        let remaining = self.commands.len().saturating_sub(*index);
        let take = remaining.min(requested as usize);
        unsafe {
            if !fetched.is_null() { *fetched = take as u32; }
            for i in 0..requested as usize {
                out.add(i).write(if i < take { Some(self.commands[*index + i].clone()) } else { None });
            }
        }
        *index += take;
        if take == requested as usize { S_OK } else { S_FALSE }
    }

    fn Skip(&self, count: u32) -> Result<()> {
        let mut index = self.index.lock().map_err(|_| fail())?;
        let remaining = self.commands.len().saturating_sub(*index);
        let step = remaining.min(count as usize);
        *index += step;
        if step == count as usize { Ok(()) } else { Err(Error::from_hresult(S_FALSE)) }
    }

    fn Reset(&self) -> Result<()> {
        *self.index.lock().map_err(|_| fail())? = 0;
        Ok(())
    }

    fn Clone(&self) -> Result<IEnumExplorerCommand> {
        Ok(CommandEnum {
            _guard: LiveGuard::take(),
            commands: self.commands.clone(),
            index: Mutex::new(*self.index.lock().map_err(|_| fail())?),
        }.into())
    }
}

#[implement(IClassFactory)]
struct ClassFactory { _guard: LiveGuard }

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(&self, outer: Ref<IUnknown>, iid: *const GUID, out: *mut *mut c_void) -> Result<()> {
        if iid.is_null() || out.is_null() { return Err(Error::from_hresult(E_POINTER)); }
        unsafe { *out = std::ptr::null_mut(); }
        if outer.is_some() { return Err(Error::from_hresult(CLASS_E_NOAGGREGATION)); }
        let root: IExplorerCommand = MenuCommand::root().into();
        unsafe { root.query(iid, out).ok() }
    }

    fn LockServer(&self, lock: BOOL) -> Result<()> {
        if lock.as_bool() {
            SERVER_LOCKS.fetch_add(1, Ordering::SeqCst);
        } else {
            let _ = SERVER_LOCKS.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| x.checked_sub(1));
        }
        Ok(())
    }
}

#[unsafe(no_mangle)]
unsafe extern "system" fn DllGetClassObject(clsid: *const GUID, iid: *const GUID, out: *mut *mut c_void) -> HRESULT {
    if clsid.is_null() || iid.is_null() || out.is_null() { return E_POINTER; }
    unsafe {
        *out = std::ptr::null_mut();
        if *clsid != COMMAND_CLSID { return CLASS_E_CLASSNOTAVAILABLE; }
        let factory: IClassFactory = ClassFactory { _guard: LiveGuard::take() }.into();
        factory.query(iid, out)
    }
}

#[unsafe(no_mangle)]
extern "system" fn DllCanUnloadNow() -> HRESULT {
    if LIVE_OBJECTS.load(Ordering::SeqCst) == 0 && SERVER_LOCKS.load(Ordering::SeqCst) == 0 { S_OK } else { S_FALSE }
}

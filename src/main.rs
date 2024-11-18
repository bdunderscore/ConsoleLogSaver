use bytemuck::{AnyBitPattern, NoUninit};
use lldb::{lldb_addr_t, lldb_offset_t, lldb_pid_t, ByteOrder, FunctionNameType, SBAttachInfo, SBData, SBDebugger, SBError, SBExpressionOptions, SBFileSpec, SBFrame, SBListener, SBModule, SBProcess, SBSymbol, SBTarget, SBValue, SymbolType};
use std::env::args;
use std::io::Write;
use std::marker::PhantomData;
use std::os::unix::fs::PermissionsExt;
use std::process::{exit, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

fn main() {
    let mut args = args();

    let unity_pid = args
        .nth(1)
        .expect("please specify pid")
        .parse::<lldb_pid_t>()
        .expect("Failed to parse unity pid");

    SBDebugger::initialize();

    let mut named_temp = tempfile::Builder::new()
        .prefix("cls-lldb-debugserver")
        .suffix(".exe")
        .tempfile()
        .expect("failed to create temporary file");

    named_temp.as_file_mut().set_permissions(std::fs::Permissions::from_mode(0o755)).expect("failed to set permissions");
    named_temp.write_all(include_bytes!("/Users/anatawa12/CLionProjects/llvm-project/build/bin/debugserver")).expect("creating debugserver failed");

    unsafe {
        std::env::set_var("LLDB_DEBUGSERVER_PATH", named_temp.path());
    }

    let debugger = SBDebugger::create(false);
    //debugger.enable_log("lldb", &[
    //    "default",
    //    //"api", 
    //    //"break",
    //    //"dyld",
    //    //"jit",
    //    //"event",
    //    //"process",
    //    "platform",
    //]);
    debugger.set_asynchronous(false);

    // reading symbol table took some time, we want to skip
    let target = debugger.create_target("", None, None, false).unwrap();

    let attach = Instant::now();
    let attach_info = SBAttachInfo::new_with_pid(unity_pid);

    let process = target.attach(attach_info).unwrap();
    println!("Attaching process took {:?}, running?: {}", attach.elapsed(), process.is_running());
    
    println!("removing temp server");
    drop(named_temp);
    //sleep(Duration::from_secs(30));

    let before_break = Instant::now();

    // I don't know wht but target.find_functions("SceneTracker::Update") don't work on windows
    // so we use different method
    let mut update = None;
    'modules: for module in target.modules() {
        if !module.filespec().filename().contains("Unity") {
            continue;
        }
        for symbol in module.symbols() {
            //println!("Processing symbol {:?}", symbol);
            if symbol.name().contains("SceneTracker::Update(") {
                update = Some(symbol.start_address().expect("no start address for SceneTracker::Update"));
                break 'modules;
            }
        }
    }

    let update = update.expect("SceneTracker::Update symbol not found");

    let breakpoint = target.breakpoint_create_by_sbaddress(update);
    breakpoint.set_enabled(true);
    breakpoint.set_oneshot(true);

    process.continue_execution().unwrap();

    println!("continue to breakpoint took {:?}", before_break.elapsed());

    if target.byte_roder() != current_byte_order() {
        eprintln!(
            "byte order mismatch (target={target:?},current={current:?})",
            target = target.byte_roder(),
            current = current_byte_order(),
        );
        exit(1);
    }

    if target.get_address_byte_size() as usize != size_of::<usize>() {
        eprintln!("pointer size mismatch");
        exit(1);
    }

    let thread = process.selected_thread();
    let frame = thread.frames().nth(0).unwrap();

    let ctx = LLDBContext::new(&target, &process, &frame);

    unsafe {
        let path = "/Users/anatawa12/RustroverProjects/console-log-saver/target/debug/libcls_attach_lib.dylib";
        let path_cstring = std::ffi::CString::new(path).unwrap();
        let dylib = SBFileSpec { raw: lldb::sys::CreateSBFileSpec2(path_cstring.as_ptr()) };
        let error = SBError::default();
        let image_token = lldb::sys::SBProcessLoadImage(process.raw, dylib.raw, error.raw);
        if (error.is_failure()) {
            sleep(Duration::from_secs(1));
            panic!("error: {}", error);
        }
        let saver_save = ctx.get_function_addr("CONSOLE_LOG_SAVER_SAVE");
        let location = ctx.get_addr("CONSOLE_LOG_SAVER_SAVED_LOCATION");
        println!("saver save address: {}", saver_save);
        println!("saver location address: {}", location);

        ctx.eval(&format!("((void (*)())({saver_save}))()"));

        let error = lldb::sys::SBProcessUnloadImage(process.raw, image_token);
        let error = SBError { raw: error };
        if error.is_failure() {
            eprintln!("error: {}", error);
        }
    }
    // GetOrCreateClangModule

    /*
    let define = Instant::now();
    ctx.eval(&format!(
        r#"
        void * (*$mono_domain_get)() = (decltype($mono_domain_get))({mono_domain_get});
        void * (*$mono_assembly_name_new)(const char *) = (decltype($mono_assembly_name_new))({mono_assembly_name_new});
        void * (*$mono_assembly_loaded)(void *) = (decltype($mono_assembly_loaded))({mono_assembly_loaded});
        void * (*$mono_assembly_get_image)(void *) = (decltype($mono_assembly_get_image))({mono_assembly_get_image});
        void * (*$mono_class_from_name)(void *, const char *, const char *) = (decltype($mono_class_from_name))({mono_class_from_name});
        void * (*$mono_class_get_field_from_name)(void *, const char *) = (decltype($mono_class_get_field_from_name))({mono_class_get_field_from_name});
        void * (*$mono_object_new)(void *, void *) = (decltype($mono_object_new))({mono_object_new});
        void (*$mono_runtime_object_init)(void *) = (decltype($mono_runtime_object_init))({mono_runtime_object_init});
        void (*$mono_field_get_value)(void *, void *, void *) = (decltype($mono_field_get_value))({mono_field_get_value});
        char16_t * (*$mono_string_chars)(void *) = (decltype($mono_string_chars))({mono_string_chars});
        int (*$mono_string_length)(void *) = (decltype($mono_string_length))({mono_string_length});
        void * (*$mono_method_desc_new)(const char *, int32_t) = (decltype($mono_method_desc_new))({mono_method_desc_new});
        void * (*$mono_method_desc_search_in_class)(void *, void *) = (decltype($mono_method_desc_search_in_class))({mono_method_desc_search_in_class});
        void * (*$mono_runtime_invoke)(void *, void *, void **, void **) = (decltype($mono_runtime_invoke))({mono_runtime_invoke});
        void * (*$mono_object_unbox)(void *) = (decltype($mono_object_unbox))({mono_object_unbox});

        // values on the mono world
        void *$domain = $mono_domain_get();
        void *$assembly_name = $mono_assembly_name_new("UnityEditor");
        void *$assembly = $mono_assembly_loaded($assembly_name);
        void *$image = $mono_assembly_get_image($assembly);

        void *$LogEntryClass = $mono_class_from_name($image, "UnityEditor", "LogEntry");
        void *$LogEntryClass_message = $mono_class_get_field_from_name($LogEntryClass, "message");
        void *$LogEntryClass_line = $mono_class_get_field_from_name($LogEntryClass, "line");
        void *$LogEntryClass_mode = $mono_class_get_field_from_name($LogEntryClass, "mode");

        void *$LogEntriesClass = $mono_class_from_name($image, "UnityEditor", "LogEntries");
        void *$StartGettingEntries = $mono_method_desc_search_in_class($mono_method_desc_new("int:StartGettingEntries()", 1), $LogEntriesClass);
        void *$EndGettingEntries = $mono_method_desc_search_in_class($mono_method_desc_new(":EndGettingEntries()", 1), $LogEntriesClass);
        void *$GetEntryInternal = $mono_method_desc_search_in_class($mono_method_desc_new(":GetEntryInternal(int,UnityEditor.LogEntry)", 1), $LogEntriesClass);
    "#,
        mono_domain_get = ctx.get_function_addr("mono_domain_get"),
        mono_assembly_name_new = ctx.get_function_addr("mono_assembly_name_new"),
        mono_assembly_loaded = ctx.get_function_addr("mono_assembly_loaded"),
        mono_assembly_get_image = ctx.get_function_addr("mono_assembly_get_image"),
        mono_class_from_name = ctx.get_function_addr("mono_class_from_name"),
        mono_class_get_field_from_name = ctx.get_function_addr("mono_class_get_field_from_name"),
        mono_object_new = ctx.get_function_addr("mono_object_new"),
        mono_runtime_object_init = ctx.get_function_addr("mono_runtime_object_init"),
        mono_field_get_value = ctx.get_function_addr("mono_field_get_value"),
        mono_string_chars = ctx.get_function_addr("mono_string_chars"),
        mono_string_length = ctx.get_function_addr("mono_string_length"),
        mono_method_desc_new = ctx.get_function_addr("mono_method_desc_new"),
        mono_method_desc_search_in_class = ctx.get_function_addr("mono_method_desc_search_in_class"),
        mono_runtime_invoke = ctx.get_function_addr("mono_runtime_invoke"),
        mono_object_unbox = ctx.get_function_addr("mono_object_unbox"),
    ));
    println!("define: {:?}", define.elapsed());

    let all_total = Instant::now();

    let main_eval = Instant::now();
    ctx.eval(r#"
        void *logentry = $mono_object_new($domain, $LogEntryClass);
        $mono_runtime_object_init(logentry);

        int32_t count = *((int32_t *)$mono_object_unbox($mono_runtime_invoke($StartGettingEntries, NULL, {}, NULL)));

        int32_t $message_length[count];
        char16_t *$message_chars[count];

        void *$message_obj;
        int $line, $mode;

        for (int32_t index = 0; index < count; index++) {
            $mono_runtime_invoke($GetEntryInternal, NULL, (void *[]){&index, logentry}, NULL);
            $mono_field_get_value(logentry, $LogEntryClass_message, &$message_obj);
            $mono_field_get_value(logentry, $LogEntryClass_line, &$line);
            $mono_field_get_value(logentry, $LogEntryClass_mode, &$mode);

            $message_length[index] = $mono_string_length($message_obj);
            $message_chars[index]  = $mono_string_chars($message_obj);
        }

        $mono_runtime_invoke($EndGettingEntries, NULL, {}, NULL);

        struct Result {
            int32_t count;
            int32_t *message_length;
            char16_t **message_chars;
        };

        struct Result $result = {
            .count = count,
            .message_length = $message_length,
            .message_chars  = $message_chars,
        };
    "#);

    println!("main_eval: {:?}", main_eval.elapsed());

    let count = ctx.eval("$result.count").get_signed().unwrap() as usize;
    let message_length = ctx.read_array::<u32>(
        count,
        ctx.eval("$result.message_length").get_signed().unwrap() as lldb_addr_t,
    );
    let message_chars = ctx.read_array::<usize>(
        count,
        ctx.eval("$result.message_chars").get_signed().unwrap() as lldb_addr_t,
    );

    for index in 0..count {
        let message_ptr = message_chars[index];
        let message_len = message_length[index];

        let mut buffer = vec![0u16; message_len as usize];
        process
            .read_memory(
                message_ptr as lldb_addr_t,
                bytemuck::cast_slice_mut(&mut buffer),
            )
            .unwrap();

        let message = String::from_utf16(buffer.as_slice()).unwrap();
        //println!("{message}");
        //println!();
    }

    println!("all_total: {:?}", all_total.elapsed());
     */

    process.detach().unwrap();

    SBDebugger::terminate();
}

struct LLDBContext<'a> {
    target: &'a SBTarget,
    process: &'a SBProcess,
    frame: &'a SBFrame,
}

impl<'a> LLDBContext<'a> {
    fn new(target: &'a SBTarget, process: &'a SBProcess, frame: &'a SBFrame) -> Self {
        Self {
            target,
            process,
            frame,
        }
    }
}

impl LLDBContext<'_> {
    fn eval(&self, expr: &str) -> SBValue {
        let options = SBExpressionOptions::new();
        let result = self.frame.evaluate_expression(expr, &options);
        // 0x1001 is kNoResult, which is not an error
        // https://github.com/llvm/llvm-project/blob/d6e65a66095cc3c93ea78669bc41d0885780e8ea/lldb/include/lldb/Expression/UserExpression.h#L274
        if result
            .error()
            .map(|x| x.is_failure() && x.error() != 0x1001)
            .unwrap_or(false)
        {
            panic!("{}", result.error().unwrap());
        }
        result
    }

    fn get_function_addr(&self, name: &str) -> u64 {
        self.target
            .find_functions(name, FunctionNameType::AUTO.bits())
            .iter()
            .nth(0)
            .unwrap()
            .symbol()
            .start_address()
            .unwrap()
            .load_address(self.target)
    }

    fn get_addr(&self, name: &str) -> u64 {
        self.target
            .find_symbols(name, SymbolType::Data)
            .iter()
            .nth(0)
            .unwrap()
            .symbol()
            .start_address()
            .unwrap()
            .load_address(self.target)
    }

    fn read_array<T: NoUninit + AnyBitPattern + Default>(
        &self,
        length: usize,
        ptr: lldb_addr_t,
    ) -> Vec<T> {
        let mut buffer = vec![T::default(); length];
        self.process
            .read_memory(ptr, bytemuck::cast_slice_mut(&mut buffer))
            .unwrap();
        buffer
    }
}

fn current_byte_order() -> ByteOrder {
    if cfg!(target_endian = "little") {
        ByteOrder::Little
    } else if cfg!(target_endian = "big") {
        ByteOrder::Big
    } else {
        ByteOrder::Invalid
    }
}

enum MethodArg<'a> {
    Object(&'a SBValue),
    #[allow(dead_code)]
    Primitive(&'a SBValue),
    Literal(i64),
}

unsafe trait SBProcessExt {
    fn raw(&self) -> lldb::sys::SBProcessRef;

    fn read_memory(&self, addr: lldb_addr_t, buffer: &mut [u8]) -> Result<(), SBError> {
        unsafe {
            let error = SBError::default();
            lldb::sys::SBProcessReadMemory(
                self.raw(),
                addr,
                buffer.as_mut_ptr() as *mut _,
                buffer.len(),
                error.raw,
            );
            if error.is_success() {
                Ok(())
            } else {
                Err(error)
            }
        }
    }

    fn byte_roder(&self) -> ByteOrder {
        unsafe { lldb::sys::SBProcessGetByteOrder(self.raw()) }
    }
}

unsafe impl SBProcessExt for SBProcess {
    fn raw(&self) -> lldb::sys::SBProcessRef {
        self.raw
    }
}

unsafe trait SBTargetExt {
    fn raw(&self) -> lldb::sys::SBTargetRef;

    fn byte_roder(&self) -> ByteOrder {
        unsafe { lldb::sys::SBTargetGetByteOrder(self.raw()) }
    }

    fn get_address_byte_size(&self) -> u32 {
        unsafe { lldb::sys::SBTargetGetAddressByteSize(self.raw()) }
    }
}

unsafe impl SBTargetExt for SBTarget {
    fn raw(&self) -> lldb::sys::SBTargetRef {
        self.raw
    }
}

unsafe trait SBDataExt {
    fn data_ref(&self) -> lldb::sys::SBDataRef;

    fn get_address(&self, offset: lldb_offset_t) -> Result<lldb_addr_t, SBError> {
        unsafe {
            let error = SBError::default();
            let result = lldb::sys::SBDataGetAddress(self.data_ref(), error.raw, offset);
            if error.is_success() {
                Ok(result)
            } else {
                Err(error)
            }
        }
    }

    fn read_raw(&self, offset: lldb_offset_t, buffer: &mut [u8]) -> Result<(), SBError> {
        unsafe {
            let error = SBError::default();
            lldb::sys::SBDataReadRawData(
                self.data_ref(),
                error.raw,
                offset,
                buffer.as_mut_ptr() as *mut _,
                buffer.len(),
            );
            lldb::sys::SBDataGetAddress(self.data_ref(), error.raw, offset);
            if error.is_success() {
                Ok(())
            } else {
                Err(error)
            }
        }
    }
}

unsafe impl SBDataExt for SBData {
    fn data_ref(&self) -> lldb::sys::SBDataRef {
        self.raw
    }
}

unsafe trait SBValueExt {
    fn data_ref(&self) -> lldb::sys::SBValueRef;

    fn get_signed(&self) -> Result<i64, SBError> {
        unsafe {
            let error = SBError::default();
            let result = lldb::sys::SBValueGetValueAsSigned(self.data_ref(), error.raw, 0);
            if error.is_success() {
                Ok(result)
            } else {
                Err(error)
            }
        }
    }
}

unsafe impl SBValueExt for SBValue {
    fn data_ref(&self) -> lldb::sys::SBValueRef {
        self.raw
    }
}

unsafe trait SBModuleExt {
    fn raw(&self) -> lldb::sys::SBModuleRef;

    fn symbols(&self) -> ModuleSymbols {
        ModuleSymbols {
            module: self.raw(),
            _phantom: PhantomData,
        }
    }
}

unsafe impl SBModuleExt for SBModule {
    fn raw(&self) -> lldb::sys::SBModuleRef {
        self.raw
    }
}

struct ModuleSymbols<'a> {
    module: lldb::sys::SBModuleRef,
    _phantom: PhantomData<&'a SBModule>,
}

impl ModuleSymbols<'_> {
    pub fn len(&self) -> usize {
        unsafe { lldb::sys::SBModuleGetNumSymbols(self.module) }
    }

    pub fn get(&self, index: usize) -> Option<SBSymbol> {
        if index < self.len() {
            let symbol = unsafe { lldb::sys::SBModuleGetSymbolAtIndex(self.module, index) };
            Some(SBSymbol { raw: symbol })
        } else {
            None
        }
    }
}

impl<'a> IntoIterator for ModuleSymbols<'a> {
    type Item = SBSymbol;
    type IntoIter = ModuleSymbolsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ModuleSymbolsIter {
            module: self,
            index: 0,
        }
    }
}

struct ModuleSymbolsIter<'a> {
    module: ModuleSymbols<'a>,
    index: usize,
}

impl Iterator for ModuleSymbolsIter<'_> {
    type Item = SBSymbol;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.module.len() {
            self.index += 1;
            self.module.get(self.index - 1)
        } else {
            None
        }
    }
}

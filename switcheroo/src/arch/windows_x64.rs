/// After the call to this function the stack should look like this:\
/// * Deallocation stack.
/// * Stack limit.
/// * Stack base.
/// * Stack frame pointer. Can be zero. The swap function preservs the previous one and the next function
///   can set it, but most modern compilers don't use it, instead they use the rbp as general purpos reg.
/// * Pointer to the function we want to call
/// * Return address. Can also be zero, as we never return from this stack. But the ABI expects it for
///   alignement reasons: "the value (%rsp + 8) is always a multiple of 16 when control is transferred to
///   the function entry point".
/// -----------------------------------------------------------------------------------------------------
/// *
/// * The Home addresses are required for at least 4 arguments by Windows:
/// * https://docs.microsoft.com/en-us/cpp/build/stack-usage?view=vs-2019
/// *
///
/// Returns a pointer to the new top of the stack.
/// The `swap` function will pop the first few values to set up the Thread Information Block and function
/// call.
pub unsafe fn init<S: stackpp::Stack>(stack: &S, f: unsafe extern "C" fn(usize, *mut u8) -> !) -> *mut u8 {
    let old_bottom = stack.bottom() as *mut usize;
    let bottom = old_bottom;
    // Home addresses - zeroed
    let bottom = bottom.offset(-4);
    // Return address - zeroed
    let bottom = bottom.offset(-1);

    // Pointer to the function we want to call
    let bottom = bottom.offset(-1);
    *bottom = f as usize;
    // Stack frame pointer - zeroed
    let bottom = bottom.offset(-1);

    // The next few values are not really documented in windows and we rely on this Wiki page:
    // https://en.wikipedia.org/wiki/Win32_Thread_Information_Block
    // and this file from Boost's Context library:
    // https://github.com/boostorg/context/blob/develop/src/asm/jump_x86_64_ms_pe_masm.asm
    // to preserve all needed information for Windows to be able to automatically move the stack guard page.

    // Stack base
    let bottom = bottom.offset(-1);
    *bottom = old_bottom as usize;

    // Stack limit, 4 pages under the guard on Windows.
    // TODO: In this case we don't need the top field in the struct and can delete it.
    let bottom = bottom.offset(-1);
    *bottom = stack.top() as usize;

    // Deallocation stack, where the actual memory address of the stack starts.
    // There are a few pages between the limit and here for the exception handler to have enough stack in case
    // of a stack overflow exception.
    let bottom = bottom.offset(-1);
    *bottom = stack.deallocation() as usize;

    bottom as *mut u8
}


#[inline(always)]
pub unsafe fn swap(arg: usize, new_sp: *mut u8) -> (usize, *mut u8) {
    let ret_val: usize;
    let ret_sp: *mut u8;

    asm!(
        // Save the continuation spot after we jump back here to be after this asm block.
        "lea rax, [rip + 1337f]",
        "push rax",
        // Save the frame pointer as it can't be marked as an output register.
        "push rbp",

        // Load NT_TIB
        "mov r10, gs:[030h]",
        // Save stack base
        "mov rax, [r10+08h]",
        "push rax",
        // Save stack limit
        "mov rax, [r10+010h]",
        "push rax",
        // Save deallocation stack
        "mov rax, [r10+01478h]",
        "push rax",

        // Set the current pointer as the 2nd element (rdx) of the function we are jumping to.
        "mov rdx, rsp",
        // Change the stack pointer to the passed value.
        "mov rsp, rsi",

        // Set deallocation stack
        "pop rax",
        "mov  [r10+01478h], rax",
        // Set stack limit
        "pop rax",
        "mov  [r10+010h], rax",
        // Set stack base
        "pop rax",
        "mov  [r10+08h], rax",

        // Set the frame pointer according to the new stack.
        "pop rbp",
        // Get the next instruction to jump to.
        "pop rax",
        // Doing a pop & jmp instad of a ret helps us here with brench prediction (3x faster on my machine).
        "jmp rax",
        "1337:",
        // Mark all registers as clobbered as we don't know what the code we are jumping to is going to use.
        // The compiler will optimise this out and just save the registers it actually knows it must.
        in("rsi") new_sp,
        inout("rcx") arg => ret_val, // 1st argument to called function
        out("rdx") ret_sp, // 2nd argument to called function
        out("rax") _, out("rbx") _, out("rdi") _, lateout("rsi") _,

        out("r8") _, out("r9") _, out("r10") _, out("r11") _,
        out("r12") _, out("r13") _, out("r14") _, out("r15") _,

        out("xmm0") _, out("xmm1") _, out("xmm2") _, out("xmm3") _,
        out("xmm4") _, out("xmm5") _, out("xmm6") _, out("xmm7") _,
        out("xmm8") _, out("xmm9") _, out("xmm10") _, out("xmm11") _,
        out("xmm12") _, out("xmm13") _, out("xmm14") _, out("xmm15") _,

        options(preserves_flags)
    );

    (ret_val, ret_sp)
}
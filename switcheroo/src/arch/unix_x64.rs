/// Pushes 2 arguments to the stack:
/// 1. Pointer to the function we want to call.
/// 2. Stack frame pointer.
/// Returns a pointer to the new top of the stack (second element).
/// The `switch` function will pop this values to set up the function call.
pub unsafe fn init<S: stackpp::Stack>(stack: &S, f: unsafe extern "C" fn(usize, *mut u8) -> !) -> *mut u8 {
    let bottom = stack.bottom() as *mut usize;
    // Aligne stack (I don't really understand this alignment, because now the address is actually not divisible by 16 bytes)
    let bottom = bottom.offset(-1);

    let first_stack_argument = bottom.offset(-1);
    *first_stack_argument = f as usize;
    let second_stack_argument = first_stack_argument.offset(-1);
    *second_stack_argument = bottom as usize;
    second_stack_argument as *mut u8
}


/// Swap between two stacks.
/// `new_sp` is the stack we are jumping to. This stack needs to have at the top:
/// 1. Stack frame pointer
/// 2. Pointer to the next instruction to execute on the new stack
/// If the pointer points to an `extern "C"` function then the `arg` element is forwarded to it
/// through the `rdi` register.
///
/// This function also pushes the stack pointer and next instruction to the current stack.
/// When we jump back to it, it will return the content of the new `arg` as ret_val.
/// TODO: Document in more detail the exact flow as this is super confusing.
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
        // Set the current pointer as the 2nd element (rsi) of the function we are jumping to.
        "mov rsi, rsp",
        // Change the stack pointer to the passed value.
        "mov rsp, rdx",
        // Set the frame pointer according to the new stack.
        "pop rbp",
        // Get the next instruction to jump to.
        "pop rax",
        // Doing a pop & jmp instad of a ret helps us here with brench prediction (3x faster on my machine).
        "jmp rax",
        "1337:",
        // Mark all registers as clobbered as we don't know what the code we are jumping to is going to use.
        // The compiler will optimise this out and just save the registers it actually knows it must.
        in("rdx") new_sp,
        inout("rdi") arg => ret_val, // 1st argument to called function
        out("rsi") ret_sp, // 2nd argument to called function
        out("rax") _, out("rbx") _, out("rcx") _, lateout("rdx") _,

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
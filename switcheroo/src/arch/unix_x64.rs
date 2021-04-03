use crate::stack;

pub unsafe fn init<S: stack::Stack>(
    stack: &S,
    f: unsafe extern "C" fn(usize, *mut usize),
) -> *mut usize {
    unsafe fn push(mut sp: *mut usize, val: usize) -> *mut usize {
        sp = sp.offset(-1);
        *sp = val;
        sp
    }

    let mut sp = stack.bottom();
    // Align stack
    sp = push(sp, 0);
    // Save the (generator_wrapper) function on the stack.
    sp = push(sp, f as usize);

    #[naked]
    unsafe extern "C" fn trampoline_1() {
        asm!(
            ".cfi_def_cfa rbp, 16",
            ".cfi_offset rbp, -16",
            "nop",
            "nop",
            options(noreturn)
        )
    }

    // Call frame for trampoline_2. The CFA slot is updated by swap::trampoline
    // each time a context switch is performed.
    sp = push(sp, trampoline_1 as usize + 2); // Point to return instruction after 2 x nop
    sp = push(sp, 0xdeaddeaddead0cfa);

    #[naked]
    unsafe extern "C" fn trampoline_2() {
        asm!(
            // call frame address = take address from register RBP and add offset 16 to it.
            // RBP at this point contains the address of the *Caller frame* stack location.
            // The *Caller frame* location contains the value of the previous RSP.
            ".cfi_def_cfa rbp, 16",
            // previous value of RBP is saved at CFA + 16
            ".cfi_offset rbp, -16",
            "nop",
            "call [rsp + 16]",
            options(noreturn)
        )
    }

    // Save frame pointer
    let frame = sp;
    sp = push(sp, trampoline_2 as usize + 1); // call instruction
    sp = push(sp, frame as usize);

    sp
}

#[inline(always)]
pub unsafe fn swap_and_link_stacks(
    arg: usize,
    new_sp: *mut usize,
    sp: *const usize,
) -> (usize, *mut usize) {
    let ret_val: usize;
    let ret_sp: *mut usize;

    asm!(
        // Save the continuation spot after we jump back here to be after this asm block.
        "lea rax, [rip + 1337f]",
        "push rax",
        // Save the frame pointer as it can't be marked as an output register.
        "push rbp",
        // Link stacks by swapping the CFA value
        "mov [rcx - 32], rsp",
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
        inout("rcx") sp => _,
        inout("rdx") new_sp => _,
        inout("rdi") arg => ret_val, // 1st argument to called function
        out("rsi") ret_sp, // 2nd argument to called function
        out("rax") _, out("rbx") _,

        out("r8") _, out("r9") _, out("r10") _, out("r11") _,
        out("r12") _, out("r13") _, out("r14") _, out("r15") _,

        out("xmm0") _, out("xmm1") _, out("xmm2") _, out("xmm3") _,
        out("xmm4") _, out("xmm5") _, out("xmm6") _, out("xmm7") _,
        out("xmm8") _, out("xmm9") _, out("xmm10") _, out("xmm11") _,
        out("xmm12") _, out("xmm13") _, out("xmm14") _, out("xmm15") _,
    );

    (ret_val, ret_sp)
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
pub unsafe fn swap(arg: usize, new_sp: *mut usize) -> (usize, *mut usize) {
    let ret_val: usize;
    let ret_sp: *mut usize;

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
        inout("rdx") new_sp => _,
        inout("rdi") arg => ret_val, // 1st argument to called function
        out("rsi") ret_sp, // 2nd argument to called function
        out("rax") _, out("rbx") _, out("rcx") _,

        out("r8") _, out("r9") _, out("r10") _, out("r11") _,
        out("r12") _, out("r13") _, out("r14") _, out("r15") _,

        out("xmm0") _, out("xmm1") _, out("xmm2") _, out("xmm3") _,
        out("xmm4") _, out("xmm5") _, out("xmm6") _, out("xmm7") _,
        out("xmm8") _, out("xmm9") _, out("xmm10") _, out("xmm11") _,
        out("xmm12") _, out("xmm13") _, out("xmm14") _, out("xmm15") _,
    );

    (ret_val, ret_sp)
}

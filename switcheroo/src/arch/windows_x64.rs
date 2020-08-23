use crate::stack;

/// After the call to this function the stack should look like this:\
/// * Deallocation stack.
/// * Stack limit.
/// * Stack base.
/// * Stack frame pointer. Will be overwritten by `swap_and_link`.
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
    // Add 2 slots of shadow stack on Windows + align stack. With the other arguments pushed later
    // this will result in actually 4 slots of shadow stack, as required by Windwos.
    for _ in 0..3 {
        sp = push(sp, 0);
    }
    // Save the function on the stack that is going to be called by ``
    sp = push(sp, f as usize);

    #[naked]
    unsafe extern "C" fn trampoline_1() {
        asm!("nop", "nop",)
    }

    // Call frame for trampoline_2. The CFA slot is updated by swap_and_link function
    // each time a context switch is performed.
    sp = push(sp, trampoline_1 as usize + 2); // Point to return instruction after 2 x nop
    sp = push(sp, 0xdeaddeaddead0cfa);

    #[naked]
    unsafe extern "C" fn trampoline_2() {
        asm!("nop", "call [rsp + 16]",)
    }

    // Save frame pointer
    let frame = sp;
    sp = push(sp, trampoline_2 as usize + 1); // call instruction
    sp = push(sp, frame as usize);

    // The next few values are not really documented in windows and we rely on this Wiki page:
    // https://en.wikipedia.org/wiki/Win32_Thread_Information_Block
    // and this file from Boost's Context library:
    // https://github.com/boostorg/context/blob/develop/src/asm/jump_x86_64_ms_pe_masm.asm
    // to preserve all needed information for Windows to be able to automatically move the stack guard page.

    // Stack base
    sp = push(sp, stack.bottom() as usize);

    // Stack limit, 4 pages under the guard on Windows.
    sp = push(sp, stack.top() as usize);

    // Deallocation stack, where the actual memory address of the stack starts.
    // There are a few pages between the limit and here for the exception handler to have enough stack in case
    // of a stack overflow exception.
    sp = push(sp, stack.deallocation() as usize);

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

        // Link stacks
        "mov [rdi-48], rsp",

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
        in("rdi") sp => _,
        in("rsi") new_sp => _,
        inout("rcx") arg => ret_val, // 1st argument to called function
        out("rdx") ret_sp, // 2nd argument to called function
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
        in("rsi") new_sp =>,
        inout("rcx") arg => ret_val, // 1st argument to called function
        out("rdx") ret_sp, // 2nd argument to called function
        out("rax") _, out("rbx") _, out("rdi") _,

        out("r8") _, out("r9") _, out("r10") _, out("r11") _,
        out("r12") _, out("r13") _, out("r14") _, out("r15") _,

        out("xmm0") _, out("xmm1") _, out("xmm2") _, out("xmm3") _,
        out("xmm4") _, out("xmm5") _, out("xmm6") _, out("xmm7") _,
        out("xmm8") _, out("xmm9") _, out("xmm10") _, out("xmm11") _,
        out("xmm12") _, out("xmm13") _, out("xmm14") _, out("xmm15") _,
    );

    (ret_val, ret_sp)
}

use crate::stack;
use core::arch::asm;

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

    // Save the (generator_wrapper) function on the stack.
    sp = push(sp, f as usize);
    sp = push(sp, 0xdeaddeaddead0cfa);

    #[naked]
    #[no_mangle]
    unsafe extern "C" fn trampoline() {
        asm!(
            // This directives will create unwind codes to link the two stacks together during stack traces.
            // The assembly was carefully crafted by a painfully long process of trial and error. For the most
            // part I was guessing how the stack tracing uses the Windows unwind codes and then went ahead and
            // constructed appropriate seh_* directives to generate this unwind codes. The desired outcome can
            // be described in different ways with seh_* directives, but after many tests this was established
            // to be the most reliable one under debug and release builds. The produced unwind codes are:
            //
            // 0x04: UOP_PushNonVol RSP - Restore the RSP by pointing it to the previous stack and increment it
            //                            by 8, jumping over the stack place holding the the deallocation stack.
            // 0x03: UOP_AllocSmall 16  - Increment the RSP by 16 jumping over 2 stack slots: stack limit & base.
            // 0x02: UOP_PushNonVol RBX - Restore RBX register that is used internally by LLVM and can't be
            //                            marked as clobbered.
            // 0x01: UOP_PushNonVol RBP - Pop the previous RBP from the stack.
            //
            // Once the unwinder reaches this function the value on the stack is going to be the value of the
            // previous RSP. After it processes the unwind codes it will look like `trampoline` was called from
            // the `swap` function, because the next value on the stack is the IP value pointing back inside
            // `swap`.
            //
            // Opposite of Unix systems, here we only need one trampoline function to achieve the same outcome.
            //
            // NOTE: To get the unwind codes from a Windows executable run:
            // 1. rabin2.exe -P .\target\debug\examples\async.pdb > pdb.txt
            // 2. Search inside the pdb.txt file to locate the `trampoline` function and note the address.
            // 3. llvm-objdump -u target\debug\examples\async.exe > unwind_info.txt
            // 4. Use the address from step 2 to locate the unwind codes of the `trampline` function.
            //
            // TODO: Create ASCII art showing how exactly the stack looks.
            ".seh_proc trampoline",
            "nop",
            ".seh_pushreg rbp",
            "nop",
            ".seh_pushreg rbx",
            "nop",
            ".seh_stackalloc 16",
            "nop",
            ".seh_pushreg rsp",
            ".seh_endprologue",
            "call [rsp + 8]",
            "nop",
            "nop",
            ".seh_endproc",
            options(noreturn)
        )
    }

    // Save frame pointer
    let frame = sp;
    sp = push(sp, trampoline as usize + 4); //  "call [rsp + 8]" instruction
    sp = push(sp, frame as usize);

    // Set rbx starting value to 0
    sp = push(sp, 0);

    // The next few values are not really documented in Windows and we rely on this Wiki page:
    // https://en.wikipedia.org/wiki/Win32_Thread_Information_Block
    // and this file from Boost's Context library:
    // https://github.com/boostorg/context/blob/develop/src/asm/jump_x86_64_ms_pe_masm.asm
    // to preserve all needed information for Windows to be able to automatically extend the stack and
    // move the stack guard page.

    // Stack base
    sp = push(sp, stack.bottom() as usize);

    // Stack limit, 4 pages under the deallocation stack on Windows.
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
        // rbx is is used internally by LLVM and can't be marked as an output register.
        "push rbx",

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
        "mov [rdi - 16], rsp",

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

        // Restore rbx
        "pop rbx",
        // Set the frame pointer according to the new stack.
        "pop rbp",
        // Get the next instruction to jump to.
        "pop rax",
        // Doing a pop & jmp instad of a ret helps us here with brench prediction (3x faster on my machine).
        "jmp rax",
        "1337:",
        // Mark all registers as clobbered as we don't know what the code we are jumping to is going to use.
        // The compiler will optimise this out and just save the registers it actually knows it must.
        inout("rdi") sp => _,
        inout("rsi") new_sp => _,
        inout("rcx") arg => ret_val, // 1st argument to called function
        out("rdx") ret_sp, // 2nd argument to called function
        out("rax") _,

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
        // rbx is is used internally by LLVM can't be marked as an output register.
        "push rbx",

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

        // Restore rbx
        "pop rbx",
        // Set the frame pointer according to the new stack.
        "pop rbp",
        // Get the next instruction to jump to.
        "pop rax",
        // Doing a pop & jmp instad of a ret helps us here with brench prediction (3x faster on my machine).
        "jmp rax",
        "1337:",
        // Mark all registers as clobbered as we don't know what the code we are jumping to is going to use.
        // The compiler will optimise this out and just save the registers it actually knows it must.
        inout("rsi") new_sp => _,
        inout("rcx") arg => ret_val, // 1st argument to called function
        out("rdx") ret_sp, // 2nd argument to called function
        out("rax") _,  out("rdi") _,

        out("r8") _, out("r9") _, out("r10") _, out("r11") _,
        out("r12") _, out("r13") _, out("r14") _, out("r15") _,

        out("xmm0") _, out("xmm1") _, out("xmm2") _, out("xmm3") _,
        out("xmm4") _, out("xmm5") _, out("xmm6") _, out("xmm7") _,
        out("xmm8") _, out("xmm9") _, out("xmm10") _, out("xmm11") _,
        out("xmm12") _, out("xmm13") _, out("xmm14") _, out("xmm15") _,
    );

    (ret_val, ret_sp)
}

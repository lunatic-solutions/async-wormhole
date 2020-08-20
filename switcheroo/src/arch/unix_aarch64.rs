use crate::stack;

pub unsafe fn init<S: stack::Stack>(
    stack: &S,
    f: unsafe extern "C" fn(usize, *mut usize) -> !,
) -> *mut usize {
    unsafe fn push(mut sp: *mut usize, val: usize) -> *mut usize {
        sp = sp.offset(-1);
        *sp = val;
        sp
    }

    let mut sp = stack.bottom();
    // Align stack
    sp = push(sp, 0);
    // Save the function on the stack that is going to be called by ``
    sp = push(sp, f as usize);

    #[naked]
    unsafe extern "C" fn trampoline_1() {
        asm!(
            ".cfi_def_cfa x29, 16",
            ".cfi_offset x30, -8",
            ".cfi_offset x29, -16",
            "nop",)
    }

    // Call frame for trampoline_2. The CFA slot is updated by swap::trampoline
    // each time a context switch is performed.
    sp = push(sp, trampoline_1 as usize + 4); // Point to return instruction after 2 x nop
    sp = push(sp, 0xdeaddeaddead0cfa);

    #[naked]
    unsafe extern "C" fn trampoline_2() {
        asm!(
            ".cfi_def_cfa x29, 16",
            ".cfi_offset x30, -8",
            ".cfi_offset x29, -16",
            "nop",
            "ldr x2, [sp, #16]",
            "blr x2"
        )
    }

    // Save frame pointer
    let frame = sp;
    sp = push(sp, trampoline_2 as usize + 4); // call instruction
    sp = push(sp, frame as usize);

    sp
}

#[inline(always)]
pub unsafe fn swap_and_link_stacks(
    arg: usize,
    new_sp: *mut usize,
    sp: *const usize,
) -> (usize, *mut usize) {
    let cfa_of_caller = sp.offset(-4);

    let ret_val: usize;
    let ret_sp: *mut usize;

    asm!(
        "adr lr, 1337f",
        "stp x29, x30, [sp, #-16]!",
        "mov x1, sp",
        "str x1, [x3]",
        "mov sp, x2",
        "ldp x29, x30, [sp], #16",
        "br x30",
        "1337:",

        inout("x3") cfa_of_caller => _,
        inout("x2") new_sp => _,
        inout("x0") arg => ret_val,
        out("x1") ret_sp,

        out("x4") _, out("x5") _, out("x6") _, out("x7") _,
        out("x8") _, out("x9") _, out("x10") _, out("x11") _,
        out("x12") _, out("x13") _, out("x14") _, out("x15") _,
        out("x16") _, out("x17") _, out("x18") _, out("x19") _,
        out("x20") _, out("x21") _, out("x22") _, out("x23") _,
        out("x24") _, out("x25") _, out("x26") _, out("x27") _,
    );

    (ret_val, ret_sp)
}

#[inline(always)]
pub unsafe fn swap(arg: usize, new_sp: *mut usize) -> (usize, *mut usize) {
    let ret_val: usize;
    let ret_sp: *mut usize;

    asm!(
        "adr lr, 1337f",
        "stp x29, x30, [sp, #-16]!",
        "mov x1, sp",
        "mov sp, x2",
        "ldp x29, x30, [sp], #16",
        "br x30",
        "1337:",

        inout("x2") new_sp => _,
        inout("x0") arg => ret_val,
        out("x1") ret_sp, out("x3") _,

        out("x4") _, out("x5") _, out("x6") _, out("x7") _,
        out("x8") _, out("x9") _, out("x10") _, out("x11") _,
        out("x12") _, out("x13") _, out("x14") _, out("x15") _,
        out("x16") _, out("x17") _, out("x18") _, out("x19") _,
        out("x20") _, out("x21") _, out("x22") _, out("x23") _,
        out("x24") _, out("x25") _, out("x26") _, out("x27") _,
        out("x28") _,
    );

    (ret_val, ret_sp)
}

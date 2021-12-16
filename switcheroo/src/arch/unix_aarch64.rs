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
    unsafe extern "C" fn trampoline() {
        asm!(
            // Stops unwinding/backtracing at this function.
            ".cfi_undefined lr",
            "ldr x2, [sp, #8]",
            "blr x2",
            options(noreturn)
        )
    }

    // Save frame pointer
    let frame = sp;
    sp = push(sp, trampoline as usize);
    sp = push(sp, frame as usize);

    // x18 & x 19
    sp = push(sp, 0); sp = push(sp, 0);

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
        "adr lr, 1337f",
        "stp x29, x30, [sp, #-16]!",
        "stp x18, x19, [sp, #-16]!",
        "mov x1, sp",
        "str x1, [x3, #-32]",
        "mov sp, x2",
        "ldp x18, x19, [sp], #16",
        "ldp x29, x30, [sp], #16",
        "br x30",
        "1337:",

        inout("x3") sp => _,
        inout("x2") new_sp => _,
        inout("x0") arg => ret_val,
        out("x1") ret_sp,

        out("x4") _, out("x5") _, out("x6") _, out("x7") _,
        out("x8") _, out("x9") _, out("x10") _, out("x11") _,
        out("x12") _, out("x13") _, out("x14") _, out("x15") _,
        out("x16") _, out("x17") _,
        out("x20") _, out("x21") _, out("x22") _, out("x23") _,
        out("x24") _, out("x25") _, out("x26") _, out("x27") _,
        out("x28") _, out("lr") _,

        out("v0") _, out("v1") _, out("v2") _, out("v3") _,
        out("v4") _, out("v5") _, out("v6") _, out("v7") _,
        out("v8") _, out("v9") _, out("v10") _, out("v11") _,
        out("v12") _, out("v13") _, out("v14") _, out("v15") _,
        out("v16") _, out("v17") _, out("v18") _, out("v19") _,
        out("v20") _, out("v21") _, out("v22") _, out("v23") _,
        out("v24") _, out("v25") _, out("v26") _, out("v27") _,
        out("v28") _, out("v29") _, out("v30") _, out("v31") _,
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
        "stp x18, x19, [sp, #-16]!",
        "mov x1, sp",
        "mov sp, x2",
        "ldp x18, x19, [sp], #16",
        "ldp x29, x30, [sp], #16",
        "br x30",
        "1337:",

        inout("x2") new_sp => _,
        inout("x0") arg => ret_val,
        out("x1") ret_sp, out("x3") _,

        out("x4") _, out("x5") _, out("x6") _, out("x7") _,
        out("x8") _, out("x9") _, out("x10") _, out("x11") _,
        out("x12") _, out("x13") _, out("x14") _, out("x15") _,
        out("x16") _, out("x17") _,
        out("x20") _, out("x21") _, out("x22") _, out("x23") _,
        out("x24") _, out("x25") _, out("x26") _, out("x27") _,
        out("x28") _, out("lr") _,

        out("v0") _, out("v1") _, out("v2") _, out("v3") _,
        out("v4") _, out("v5") _, out("v6") _, out("v7") _,
        out("v8") _, out("v9") _, out("v10") _, out("v11") _,
        out("v12") _, out("v13") _, out("v14") _, out("v15") _,
        out("v16") _, out("v17") _, out("v18") _, out("v19") _,
        out("v20") _, out("v21") _, out("v22") _, out("v23") _,
        out("v24") _, out("v25") _, out("v26") _, out("v27") _,
        out("v28") _, out("v29") _, out("v30") _, out("v31") _,
    );

    (ret_val, ret_sp)
}

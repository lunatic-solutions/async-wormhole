__attribute__((naked)) void switcheroo_trampoline() {
  asm(
    ".cfi_undefined rip\n"
    "call *8(%rsp)\n"
  );
}

/* 0043e501 __security_check_cookie */

/* WARNING: This is an inlined function */

void __fastcall __security_check_cookie(uintptr_t _StackCookie)

{
  if (_StackCookie == DAT_00451a00) {
    return;
  }
  report_failure();
  return;
}

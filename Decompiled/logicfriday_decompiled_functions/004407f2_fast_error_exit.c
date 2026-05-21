/* 004407f2 fast_error_exit */

/* Library Function - Single Match
    _fast_error_exit
   
   Library: Visual Studio 2003 Release */

void __cdecl fast_error_exit(DWORD param_1)

{
  if (DAT_0046c560 == 1) {
    __FF_MSGBANNER();
  }
  FUN_004457a6(param_1);
  ___crtExitProcess(0xff);
  return;
}

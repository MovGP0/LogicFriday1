/* 00446921 ___crtInitCritSecAndSpinCount */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */
/* Library Function - Single Match
    ___crtInitCritSecAndSpinCount
   
   Library: Visual Studio 2003 Release */

void __cdecl ___crtInitCritSecAndSpinCount(undefined4 param_1,undefined4 param_2)

{
  HMODULE hModule;
  
  if (DAT_0046c934 == (code *)0x0) {
    if (DAT_0046c6e0 != 1) {
      hModule = GetModuleHandleA("kernel32.dll");
      if (hModule != (HMODULE)0x0) {
        DAT_0046c934 = GetProcAddress(hModule,"InitializeCriticalSectionAndSpinCount");
        if (DAT_0046c934 != (FARPROC)0x0) goto LAB_0044696d;
      }
    }
    DAT_0046c934 = ___crtInitCritSecNoSpinCount_8;
  }
LAB_0044696d:
  (*DAT_0046c934)(param_1,param_2);
  return;
}

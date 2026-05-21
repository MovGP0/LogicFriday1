/* 00442e9b FUN_00442e9b */

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void __cdecl FUN_00442e9b(UINT param_1,int param_2,int param_3)

{
  HANDLE hProcess;
  undefined4 *puVar1;
  bool bVar2;
  UINT uExitCode;
  
  __lock(8);
  if (DAT_0046c720 == 1) {
    uExitCode = param_1;
    hProcess = GetCurrentProcess();
    TerminateProcess(hProcess,uExitCode);
  }
  _DAT_0046c71c = 1;
  DAT_0046c718 = (undefined1)param_3;
  if (param_2 == 0) {
    if (DAT_0046cd48 != (undefined4 *)0x0) {
      DAT_0046cd44 = DAT_0046cd44 + -1;
      bVar2 = DAT_0046cd44 < DAT_0046cd48;
      while (!bVar2) {
        if ((code *)*DAT_0046cd44 != (code *)0x0) {
          (*(code *)*DAT_0046cd44)();
        }
        DAT_0046cd44 = DAT_0046cd44 + -1;
        bVar2 = DAT_0046cd44 < DAT_0046cd48;
      }
    }
    puVar1 = &DAT_00451044;
    do {
      if ((code *)*puVar1 != (code *)0x0) {
        (*(code *)*puVar1)();
      }
      puVar1 = puVar1 + 1;
    } while (puVar1 < &DAT_0045104c);
  }
  puVar1 = &DAT_00451050;
  do {
    if ((code *)*puVar1 != (code *)0x0) {
      (*(code *)*puVar1)();
    }
    puVar1 = puVar1 + 1;
  } while (puVar1 < &DAT_00451058);
  if (param_3 == 0) {
    DAT_0046c720 = 1;
    ___crtExitProcess(param_1);
  }
  else {
    FUN_00441cd6(8);
  }
  return;
}

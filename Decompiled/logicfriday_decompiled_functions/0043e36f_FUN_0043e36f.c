/* 0043e36f FUN_0043e36f */

int FUN_0043e36f(undefined4 param_1,undefined4 param_2,undefined4 param_3,undefined4 param_4)

{
  bool bVar1;
  undefined3 extraout_var;
  int iVar2;
  BYTE local_108 [260];
  
  if ((DAT_0046c520 == (HMODULE)0x0) && (DAT_0046c528 == 0)) {
    bVar1 = FUN_0043e408(local_108);
    if (CONCAT31(extraout_var,bVar1) != 0) {
      DAT_0046c520 = LoadLibraryA((LPCSTR)local_108);
    }
    if (DAT_0046c520 != (HMODULE)0x0) goto LAB_0043e3cc;
    DAT_0046c520 = LoadLibraryA("hhctrl.ocx");
    if (DAT_0046c520 != (HMODULE)0x0) goto LAB_0043e3cc;
LAB_0043e3e7:
    DAT_0046c528 = 1;
    iVar2 = 0;
  }
  else {
LAB_0043e3cc:
    if (DAT_0046c530 == (FARPROC)0x0) {
      DAT_0046c530 = GetProcAddress(DAT_0046c520,(LPCSTR)0xe);
      if (DAT_0046c530 == (FARPROC)0x0) goto LAB_0043e3e7;
    }
    iVar2 = (*DAT_0046c530)(param_1,param_2,param_3,param_4);
  }
  return iVar2;
}

/* 004013e5 FUN_004013e5 */

undefined4 __cdecl FUN_004013e5(HWND param_1,int param_2)

{
  int iVar1;
  char *lpFile;
  HINSTANCE pHVar2;
  HWND hWnd;
  undefined4 uVar3;
  LPCSTR lpParameters;
  LPCSTR lpDirectory;
  HRGN hRgn;
  INT nShowCmd;
  BOOL bErase;
  
  iVar1 = FUN_00401000(param_2);
  if (iVar1 == 0) {
    uVar3 = 1;
  }
  else {
    nShowCmd = 0;
    lpDirectory = (LPCSTR)0x0;
    lpParameters = (LPCSTR)0x0;
    lpFile = FUN_004010ba(param_2,1);
    pHVar2 = ShellExecuteA(param_1,"open",lpFile,lpParameters,lpDirectory,nShowCmd);
    if ((int)pHVar2 < 0x21) {
      uVar3 = 0;
    }
    else {
      bErase = 0;
      hRgn = (HRGN)0x0;
      hWnd = GetDlgItem(param_1,param_2);
      InvalidateRgn(hWnd,hRgn,bErase);
      uVar3 = 1;
    }
  }
  return uVar3;
}

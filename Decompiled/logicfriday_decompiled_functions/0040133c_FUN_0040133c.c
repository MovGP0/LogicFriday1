/* 0040133c FUN_0040133c */

void __cdecl FUN_0040133c(HWND param_1,int param_2,int param_3)

{
  int iVar1;
  char *pcVar2;
  UINT c;
  INT *lpDx;
  
  iVar1 = FUN_00401000(param_2);
  if (iVar1 != 0) {
    GetDlgItem(param_1,param_2);
    iVar1 = FUN_00401036(param_2);
    if (iVar1 == 0) {
      SetTextColor(*(HDC *)(param_3 + 0x18),0xcc0000);
    }
    else {
      SetTextColor(*(HDC *)(param_3 + 0x18),0x800080);
    }
    SetBkMode(*(HDC *)(param_3 + 0x18),1);
    lpDx = (INT *)0x0;
    pcVar2 = FUN_004010ba(param_2,0);
    c = lstrlenA(pcVar2);
    pcVar2 = FUN_004010ba(param_2,0);
    ExtTextOutA(*(HDC *)(param_3 + 0x18),DAT_0046c9e8,DAT_0046c9ec,2,(RECT *)(param_3 + 0x1c),pcVar2
                ,c,lpDx);
  }
  return;
}

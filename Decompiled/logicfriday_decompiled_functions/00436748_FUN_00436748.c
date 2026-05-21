/* 00436748 FUN_00436748 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall
FUN_00436748(void *this,HWND param_1,int param_2,short param_3,undefined4 param_4)

{
  HWND hWnd;
  uint unaff_retaddr;
  UINT Msg;
  WPARAM wParam;
  LPARAM lParam;
  uint local_1c [5];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  if (param_2 == 0x110) {
    *(undefined4 *)((int)this + 0x23c4) = param_4;
    lParam = 0;
    wParam = 0x10;
    Msg = 0xc5;
    hWnd = GetDlgItem(param_1,0x43e);
    SendMessageA(hWnd,Msg,wParam,lParam);
    FUN_0043ebd0(local_1c,(uint *)(*(int *)(*(int *)((int)this + 0x16cc) +
                                           *(int *)((int)this + 0x23c4) * 4) + 0x50));
    SetDlgItemTextA(param_1,0x43e,(LPCSTR)local_1c);
    return 1;
  }
  if (param_2 == 0x111) {
    if (param_3 == 1) {
      GetDlgItemTextA(param_1,0x43e,(LPSTR)local_1c,0x11);
      FUN_0043ebd0((uint *)(*(int *)(*(int *)((int)this + 0x16cc) + *(int *)((int)this + 0x23c4) * 4
                                    ) + 0x50),local_1c);
      EndDialog(param_1,1);
      return 1;
    }
    if (param_3 == 2) {
      EndDialog(param_1,0);
      return 1;
    }
  }
  return 0;
}

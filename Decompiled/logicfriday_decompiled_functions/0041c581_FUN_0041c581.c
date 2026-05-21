/* 0041c581 FUN_0041c581 */

undefined4 __thiscall FUN_0041c581(void *this,HWND param_1,int param_2,short param_3)

{
  HWND hWnd;
  UINT UVar1;
  BOOL bEnable;
  
  if (param_2 == 0x110) {
    if (*(int *)((int)this + 0xc0) == 0) {
      CheckRadioButton(param_1,0x425,0x426,0x425);
    }
    else {
      CheckRadioButton(param_1,0x425,0x426,0x426);
      if (*(int *)(*(int *)((int)this + 0xc) + 0x1650) == 0) {
        bEnable = 0;
        hWnd = GetDlgItem(param_1,0x425);
        EnableWindow(hWnd,bEnable);
      }
    }
    return 1;
  }
  if (param_2 == 0x111) {
    if (param_3 == 1) {
      UVar1 = IsDlgButtonChecked(param_1,0x425);
      if (UVar1 == 1) {
        *(undefined4 *)((int)this + 0xc0) = 0;
      }
      else {
        *(undefined4 *)((int)this + 0xc0) = 1;
      }
      return 0;
    }
    if (param_3 == 0x424) {
      FUN_0041a43b((int)this);
      return 1;
    }
    if (param_3 == 0x425) {
      *(undefined4 *)((int)this + 0xc0) = 0;
      return 1;
    }
    if (param_3 == 0x426) {
      *(undefined4 *)((int)this + 0xc0) = 1;
      return 1;
    }
  }
  return 0;
}

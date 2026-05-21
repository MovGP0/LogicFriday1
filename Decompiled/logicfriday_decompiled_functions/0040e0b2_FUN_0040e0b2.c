/* 0040e0b2 FUN_0040e0b2 */

undefined4 FUN_0040e0b2(HWND param_1,int param_2,short param_3)

{
  UINT UVar1;
  
  if (param_2 == 0x110) {
    return 1;
  }
  if (param_2 == 0x111) {
    if (param_3 == 1) {
      UVar1 = IsDlgButtonChecked(param_1,0x48d);
      if (UVar1 == 1) {
        DAT_00452ee8 = 0;
      }
      EndDialog(param_1,1);
      return 1;
    }
    if (param_3 == 2) {
      UVar1 = IsDlgButtonChecked(param_1,0x48d);
      if (UVar1 == 1) {
        DAT_00452ee8 = 0;
      }
      EndDialog(param_1,2);
      return 1;
    }
  }
  return 0;
}

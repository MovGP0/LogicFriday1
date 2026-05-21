/* 0040ba7d FUN_0040ba7d */

undefined4 FUN_0040ba7d(HWND param_1,int param_2,short param_3,undefined4 param_4)

{
  UINT UVar1;
  
  if (param_2 == 0x110) {
    DAT_0046c500 = (UINT *)param_4;
    SendDlgItemMessageA(param_1,0x484,0x465,0,0x20010);
    SendDlgItemMessageA(param_1,0x484,0x467,0,DAT_004519dc & 0xffff);
    SendDlgItemMessageA(param_1,0x486,0x465,0,0x10010);
    SendDlgItemMessageA(param_1,0x486,0x467,0,DAT_004519d8 & 0xffff);
    SendDlgItemMessageA(param_1,0x488,0x46f,0,0x14240);
    SendDlgItemMessageA(param_1,0x488,0x471,0,DAT_004519d4 & 0xffff);
    if (DAT_0046c4fc == 0) {
      CheckDlgButton(param_1,0x48d,0);
    }
    else {
      CheckDlgButton(param_1,0x48d,1);
    }
    CheckRadioButton(param_1,0x42a,0x42b,DAT_004519d0);
    return 1;
  }
  if (param_2 == 0x111) {
    if (param_3 == 1) {
      DAT_004519dc = GetDlgItemInt(param_1,0x483,(BOOL *)0x0,0);
      DAT_004519d8 = GetDlgItemInt(param_1,0x485,(BOOL *)0x0,0);
      DAT_004519d4 = GetDlgItemInt(param_1,0x487,(BOOL *)0x0,0);
      UVar1 = IsDlgButtonChecked(param_1,0x48d);
      DAT_0046c4fc = (uint)(UVar1 == 1);
      UVar1 = IsDlgButtonChecked(param_1,0x42a);
      if (UVar1 == 1) {
        DAT_004519d0 = 0x42a;
        DAT_0046c500[3] = 0;
      }
      else {
        UVar1 = IsDlgButtonChecked(param_1,0x408);
        if (UVar1 == 1) {
          DAT_004519d0 = 0x408;
          DAT_0046c500[3] = 1;
        }
        else {
          DAT_004519d0 = 0x42b;
          DAT_0046c500[3] = 2;
        }
      }
      DAT_0046c500[2] = DAT_0046c4fc;
      *DAT_0046c500 = DAT_004519dc;
      DAT_0046c500[1] = DAT_004519d8;
      DAT_0046c500[4] = DAT_004519d4;
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

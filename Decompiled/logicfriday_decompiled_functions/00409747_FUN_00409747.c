/* 00409747 FUN_00409747 */

undefined4 FUN_00409747(HWND param_1,int param_2,short param_3,undefined4 param_4)

{
  UINT UVar1;
  HWND pHVar2;
  BOOL BVar3;
  
  if (param_2 == 0x110) {
    DAT_0046c4cc = (UINT *)param_4;
    SendDlgItemMessageA(param_1,0x484,0x465,0,0x20010);
    SendDlgItemMessageA(param_1,0x484,0x467,0,DAT_004519c4 & 0xffff);
    SendDlgItemMessageA(param_1,0x486,0x465,0,0x10010);
    SendDlgItemMessageA(param_1,0x486,0x467,0,DAT_004519c0 & 0xffff);
    SendDlgItemMessageA(param_1,0x488,0x465,0,0x14240);
    SendDlgItemMessageA(param_1,0x488,0x467,0,DAT_004519bc & 0xffff);
    SendDlgItemMessageA(param_1,0x48a,0x465,0,0x10064);
    SendDlgItemMessageA(param_1,0x48a,0x467,0,DAT_004519b8 & 0xffff);
    if (DAT_0046c4c8 == 0) {
      CheckRadioButton(param_1,0x408,0x42a,0x408);
      BVar3 = 1;
      pHVar2 = GetDlgItem(param_1,0x489);
      EnableWindow(pHVar2,BVar3);
    }
    else {
      CheckRadioButton(param_1,0x408,0x42a,0x42a);
      BVar3 = 0;
      pHVar2 = GetDlgItem(param_1,0x489);
      EnableWindow(pHVar2,BVar3);
    }
    if (DAT_0046c4c4 == 0) {
      CheckDlgButton(param_1,0x48b,0);
    }
    else {
      CheckDlgButton(param_1,0x48b,1);
    }
    if (DAT_0046c4c0 == 0) {
      CheckDlgButton(param_1,0x48c,0);
    }
    else {
      CheckDlgButton(param_1,0x48c,1);
    }
    return 1;
  }
  if (param_2 == 0x111) {
    if (param_3 == 1) {
      DAT_004519c4 = GetDlgItemInt(param_1,0x483,(BOOL *)0x0,0);
      DAT_004519c0 = GetDlgItemInt(param_1,0x485,(BOOL *)0x0,0);
      DAT_004519bc = GetDlgItemInt(param_1,0x487,(BOOL *)0x0,0);
      DAT_004519b8 = GetDlgItemInt(param_1,0x489,(BOOL *)0x0,0);
      UVar1 = IsDlgButtonChecked(param_1,0x42a);
      DAT_0046c4c8 = (uint)(UVar1 == 1);
      UVar1 = IsDlgButtonChecked(param_1,0x48b);
      DAT_0046c4c4 = (uint)(UVar1 == 1);
      UVar1 = IsDlgButtonChecked(param_1,0x48c);
      DAT_0046c4c0 = (uint)(UVar1 == 1);
      DAT_0046c4cc[4] = DAT_0046c4c8;
      DAT_0046c4cc[5] = DAT_0046c4c4;
      DAT_0046c4cc[6] = DAT_0046c4c0;
      DAT_0046c4cc[1] = DAT_004519c4;
      DAT_0046c4cc[2] = DAT_004519c0;
      DAT_0046c4cc[3] = DAT_004519bc;
      *DAT_0046c4cc = DAT_004519b8;
      EndDialog(param_1,1);
      return 1;
    }
    if (param_3 == 2) {
      EndDialog(param_1,0);
      return 1;
    }
    if (param_3 == 0x408) {
      UVar1 = IsDlgButtonChecked(param_1,0x408);
      if (UVar1 == 1) {
        BVar3 = 1;
        pHVar2 = GetDlgItem(param_1,0x489);
        EnableWindow(pHVar2,BVar3);
      }
      return 1;
    }
    if (param_3 == 0x42a) {
      UVar1 = IsDlgButtonChecked(param_1,0x42a);
      if (UVar1 == 1) {
        BVar3 = 0;
        pHVar2 = GetDlgItem(param_1,0x489);
        EnableWindow(pHVar2,BVar3);
      }
      return 1;
    }
  }
  return 0;
}

/* 0040e469 FUN_0040e469 */

undefined4 FUN_0040e469(HWND param_1,int param_2,short param_3,int param_4)

{
  HWND pHVar1;
  UINT UVar2;
  BOOL BVar3;
  
  if (param_2 == 0x110) {
    if (DAT_0046c50c == 0) {
      CheckRadioButton(param_1,0x408,0x409,0x408);
    }
    else {
      CheckRadioButton(param_1,0x408,0x409,0x409);
    }
    if (DAT_0046c508 == 0) {
      CheckRadioButton(param_1,0x42b,0x42c,0x42b);
    }
    else {
      CheckRadioButton(param_1,0x42b,0x42c,0x42c);
    }
    if (param_4 == 1) {
      BVar3 = 0;
      pHVar1 = GetDlgItem(param_1,0x495);
      EnableWindow(pHVar1,BVar3);
      BVar3 = 0;
      pHVar1 = GetDlgItem(param_1,0x42b);
      EnableWindow(pHVar1,BVar3);
      BVar3 = 0;
      pHVar1 = GetDlgItem(param_1,0x42c);
      EnableWindow(pHVar1,BVar3);
      CheckRadioButton(param_1,0x42b,0x42c,0x42b);
    }
    return 1;
  }
  if (param_2 == 0x111) {
    if (param_3 == 1) {
      DAT_0046c508 = 0;
      DAT_0046c50c = 0;
      UVar2 = IsDlgButtonChecked(param_1,0x409);
      if (UVar2 == 1) {
        DAT_0046c50c = 1;
      }
      UVar2 = IsDlgButtonChecked(param_1,0x42c);
      if (UVar2 == 1) {
        DAT_0046c508 = 1;
      }
      if ((DAT_0046c50c == 0) && (DAT_0046c508 == 0)) {
        EndDialog(param_1,0x3e9);
      }
      else if ((DAT_0046c50c == 0) && (DAT_0046c508 != 0)) {
        EndDialog(param_1,0x3ea);
      }
      else if ((DAT_0046c50c == 0) || (DAT_0046c508 != 0)) {
        if ((DAT_0046c50c != 0) && (DAT_0046c508 != 0)) {
          EndDialog(param_1,0x3ec);
        }
      }
      else {
        EndDialog(param_1,0x3eb);
      }
      return 1;
    }
    if (param_3 == 2) {
      EndDialog(param_1,2);
      return 1;
    }
  }
  return 0;
}

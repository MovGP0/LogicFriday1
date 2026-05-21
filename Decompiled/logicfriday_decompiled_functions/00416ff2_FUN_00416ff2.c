/* 00416ff2 FUN_00416ff2 */

undefined4 FUN_00416ff2(HWND param_1,int param_2,short param_3,char *param_4)

{
  if (param_2 == 0x110) {
    DAT_0046c510 = param_4;
    SendDlgItemMessageA(param_1,0x418,0xc5,8,0);
    FUN_0040be0b();
    FUN_0043ed39(DAT_0046c510,&DAT_0044a700);
    SetDlgItemTextA(param_1,0x418,DAT_0046c510);
    return 1;
  }
  if (param_2 == 0x111) {
    if (param_3 == 1) {
      GetDlgItemTextA(param_1,0x418,DAT_0046c510,9);
      EndDialog(param_1,1);
      return 1;
    }
    if (param_3 == 2) {
      EndDialog(param_1,2);
      return 1;
    }
  }
  return 0;
}

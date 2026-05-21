/* 0040bd02 FUN_0040bd02 */

undefined4 FUN_0040bd02(HWND param_1,int param_2,short param_3)

{
  if (param_2 == 0x110) {
    SendDlgItemMessageA(param_1,0x475,0x465,0,0x10064);
    SendDlgItemMessageA(param_1,0x475,0x467,0,DAT_004519e0 & 0xffff);
    return 1;
  }
  if (param_2 == 0x111) {
    if (param_3 == 1) {
      DAT_004519e0 = GetDlgItemInt(param_1,0x481,(BOOL *)0x0,0);
      EndDialog(param_1,DAT_004519e0);
      return 1;
    }
    if (param_3 == 2) {
      EndDialog(param_1,-1);
      return 1;
    }
  }
  return 0;
}

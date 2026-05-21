/* 0040b947 FUN_0040b947 */

undefined4 FUN_0040b947(HWND param_1,int param_2,short param_3,undefined4 param_4)

{
  if (param_2 == 0x110) {
    DAT_0046c4f8 = (int *)param_4;
    SendDlgItemMessageA(param_1,0x3f1,0x465,0,0x20010);
    SendDlgItemMessageA(param_1,0x3f1,0x467,0,DAT_004519cc & 0xffff);
    SendDlgItemMessageA(param_1,0x3f3,0x465,0,0x10064);
    SendDlgItemMessageA(param_1,0x3f3,0x467,0,DAT_004519c8 & 0xffff);
    return 1;
  }
  if (param_2 == 0x111) {
    if (param_3 == 1) {
      DAT_004519cc = GetDlgItemInt(param_1,0x3f0,(BOOL *)0x0,0);
      DAT_004519c8 = GetDlgItemInt(param_1,0x3f2,(BOOL *)0x0,0);
      *DAT_0046c4f8 = DAT_004519cc * 0x10000 + DAT_004519c8;
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

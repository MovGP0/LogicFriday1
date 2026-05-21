/* 0040ca22 FUN_0040ca22 */

undefined4 FUN_0040ca22(HWND param_1,int param_2,short param_3)

{
  if (param_2 == 0x110) {
    return 1;
  }
  if (param_2 == 0x111) {
    if (param_3 == 2) {
      EndDialog(param_1,-1);
      return 1;
    }
    if (param_3 == 0x425) {
      EndDialog(param_1,2);
      return 1;
    }
    if (param_3 == 0x426) {
      EndDialog(param_1,1);
      return 1;
    }
    if (param_3 == 0x428) {
      EndDialog(param_1,0);
      return 1;
    }
  }
  return 0;
}

/* 00416425 FUN_00416425 */

undefined4 FUN_00416425(HWND param_1,int param_2,short param_3)

{
  if (param_2 == 0x110) {
    return 1;
  }
  if (param_2 == 0x111) {
    if (param_3 == 2) {
      EndDialog(param_1,2);
      return 1;
    }
    if (param_3 == 0x40b) {
      EndDialog(param_1,0x40b);
      return 1;
    }
    if (param_3 == 0x40c) {
      EndDialog(param_1,0x40c);
      return 1;
    }
  }
  return 0;
}

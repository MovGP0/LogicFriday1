/* 0040adb4 FUN_0040adb4 */

undefined4 FUN_0040adb4(HWND param_1,int param_2,HDC param_3,HWND param_4)

{
  undefined4 uVar1;
  
  if (param_2 == 0x8008) {
    DAT_0046c4dc = param_4;
    uVar1 = 1;
  }
  else if (DAT_0046c4dc == (HWND)0x0) {
    uVar1 = 1;
  }
  else {
    uVar1 = FUN_00435e82(DAT_0046c4dc,param_1,param_2,param_3,param_4);
  }
  return uVar1;
}

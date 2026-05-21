/* 0040ae38 FUN_0040ae38 */

undefined4 FUN_0040ae38(HWND param_1,int param_2,short param_3,void *param_4)

{
  undefined4 uVar1;
  
  if (param_2 == 0x8008) {
    DAT_0046c4e4 = param_4;
    uVar1 = 1;
  }
  else if (DAT_0046c4e4 == (void *)0x0) {
    uVar1 = 1;
  }
  else {
    uVar1 = FUN_00436748(DAT_0046c4e4,param_1,param_2,param_3,param_4);
  }
  return uVar1;
}

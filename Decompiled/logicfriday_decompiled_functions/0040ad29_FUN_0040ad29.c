/* 0040ad29 FUN_0040ad29 */

undefined4 FUN_0040ad29(HWND param_1,int param_2,uint param_3,HWND param_4)

{
  undefined4 uVar1;
  
  if (DAT_0046c4d4 == 0) {
    if (param_2 != 0x110) {
      return 1;
    }
    DAT_0046c4d0 = param_4;
    DAT_0046c4d4 = 1;
  }
  uVar1 = FUN_00418362((void *)DAT_0046c4d0[1].unused,param_1,param_2,param_3,param_4);
  return uVar1;
}

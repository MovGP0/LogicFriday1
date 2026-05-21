/* 0040afc3 FUN_0040afc3 */

LRESULT FUN_0040afc3(HWND param_1,uint param_2,uint *param_3,uint *param_4)

{
  LRESULT LVar1;
  
  if (param_2 == 0x8008) {
    DAT_0046c4f0 = param_4;
    LVar1 = 1;
  }
  else if (DAT_0046c4f0 == (uint *)0x0) {
    LVar1 = 0;
  }
  else {
    LVar1 = FUN_0042b22e(DAT_0046c4f0,param_1,param_2,param_3,param_4);
  }
  return LVar1;
}

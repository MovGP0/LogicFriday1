/* 0040b004 FUN_0040b004 */

LRESULT FUN_0040b004(HWND param_1,uint param_2,uint param_3,int *param_4)

{
  LRESULT LVar1;
  
  if (param_2 == 0x8008) {
    DAT_0046c4f4 = param_4;
    LVar1 = 1;
  }
  else if (DAT_0046c4f4 == (int *)0x0) {
    LVar1 = DefWindowProcA(param_1,param_2,param_3,(LPARAM)param_4);
  }
  else {
    LVar1 = FUN_00437b1f(DAT_0046c4f4,param_1,param_2,param_3,param_4);
  }
  return LVar1;
}

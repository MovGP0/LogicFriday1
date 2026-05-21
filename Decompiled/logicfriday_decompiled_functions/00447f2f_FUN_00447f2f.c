/* 00447f2f FUN_00447f2f */

int FUN_00447f2f(int *param_1)

{
  int *piVar1;
  int iVar2;
  
  piVar1 = (int *)*param_1;
  if (((*piVar1 == -0x1f928c9d) && (piVar1[4] == 3)) && (piVar1[5] == 0x19930520)) {
    terminate();
  }
  if ((DAT_0046c9b4 != (_func_int *)0x0) && (iVar2 = _ValidateExecute(DAT_0046c9b4), iVar2 != 0)) {
    iVar2 = (*DAT_0046c9b4)(param_1);
    return iVar2;
  }
  return 0;
}

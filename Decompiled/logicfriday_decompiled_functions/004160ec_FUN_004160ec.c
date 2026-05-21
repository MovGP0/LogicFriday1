/* 004160ec FUN_004160ec */

undefined4 __fastcall FUN_004160ec(int param_1)

{
  undefined4 uVar1;
  
  if (((*(int *)(*(int *)(param_1 + 8) + 200) == 1) &&
      (*(int *)(*(int *)(param_1 + 0xc) + 200) == 1)) &&
     (*(int *)(*(int *)(param_1 + 8) + 0xc4) == *(int *)(*(int *)(param_1 + 0xc) + 0xc4))) {
    uVar1 = 1;
  }
  else {
    uVar1 = 0;
  }
  return uVar1;
}

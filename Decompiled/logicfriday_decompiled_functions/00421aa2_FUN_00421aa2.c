/* 00421aa2 FUN_00421aa2 */

void __fastcall FUN_00421aa2(int param_1)

{
  void *pvVar1;
  
  if (1 < *(uint *)(param_1 + 0x1660)) {
    *(undefined4 *)(param_1 + 0x1660) = 1;
    pvVar1 = _realloc(*(void **)(param_1 + 0x26c),*(int *)(param_1 + 0x1660) * 0x7fff);
    *(void **)(param_1 + 0x26c) = pvVar1;
  }
  FUN_0043ebd0(*(uint **)(param_1 + 0x26c),(uint *)&DAT_0044ad26);
  return;
}

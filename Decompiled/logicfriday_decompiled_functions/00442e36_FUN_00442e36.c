/* 00442e36 FUN_00442e36 */

int FUN_00442e36(void)

{
  int iVar1;
  undefined4 *puVar2;
  
  if (PTR_FUN_00451a18 != (undefined *)0x0) {
    (*(code *)PTR_FUN_00451a18)();
  }
  iVar1 = 0;
  puVar2 = &DAT_0045102c;
  do {
    if (iVar1 != 0) {
      return iVar1;
    }
    if ((code *)*puVar2 != (code *)0x0) {
      iVar1 = (*(code *)*puVar2)();
    }
    puVar2 = puVar2 + 1;
  } while (puVar2 < &DAT_00451040);
  if (iVar1 == 0) {
    _atexit((_func_4879 *)&LAB_00445dfa);
    puVar2 = &DAT_00451000;
    do {
      if ((code *)*puVar2 != (code *)0x0) {
        (*(code *)*puVar2)();
      }
      puVar2 = puVar2 + 1;
    } while (puVar2 < &DAT_00451028);
    iVar1 = 0;
  }
  return iVar1;
}

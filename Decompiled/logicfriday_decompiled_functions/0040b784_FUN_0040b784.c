/* 0040b784 FUN_0040b784 */

bool __cdecl FUN_0040b784(int param_1)

{
  void *this;
  int iVar1;
  undefined4 *puVar2;
  undefined4 *puVar3;
  int local_8;
  
  if (*(int *)((int)DAT_004528a4 + param_1 * 0x118 + 0x110) == DAT_00452acc) {
    DAT_00452acc = 0;
  }
  this = *(void **)((int)DAT_004528a4 + param_1 * 0x118 + 0x110);
  if (this != (void *)0x0) {
    FUN_0040b86b(this,1);
  }
  while (local_8 = param_1 + 1, local_8 < DAT_004528a0) {
    puVar2 = (undefined4 *)((int)DAT_004528a4 + local_8 * 0x118);
    puVar3 = (undefined4 *)((int)DAT_004528a4 + param_1 * 0x118);
    for (iVar1 = 0x46; param_1 = local_8, iVar1 != 0; iVar1 = iVar1 + -1) {
      *puVar3 = *puVar2;
      puVar2 = puVar2 + 1;
      puVar3 = puVar3 + 1;
    }
  }
  DAT_004528a0 = DAT_004528a0 + -1;
  DAT_004528a4 = _realloc(DAT_004528a4,DAT_004528a0 * 0x118);
  return DAT_004528a4 != (void *)0x0;
}

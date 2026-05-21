/* 004286f7 FUN_004286f7 */

void __thiscall FUN_004286f7(void *this,int *param_1,HDC param_2)

{
  int local_8;
  
  MoveToEx(param_2,*(int *)param_1[0xb],*(int *)(param_1[0xb] + 4),(LPPOINT)0x0);
  for (local_8 = 1; local_8 < param_1[10]; local_8 = local_8 + 1) {
    LineTo(param_2,*(int *)(param_1[0xb] + local_8 * 0x14),
           *(int *)(param_1[0xb] + 4 + local_8 * 0x14));
  }
  if (*param_1 == 2) {
    FUN_004287c6(this,param_2,*(int *)param_1[0xb],*(int *)(param_1[0xb] + 4));
  }
  if (param_1[5] == 2) {
    FUN_004287c6(this,param_2,*(int *)(param_1[0xb] + (param_1[10] + -1) * 0x14),
                 *(int *)(param_1[0xb] + 4 + (param_1[10] + -1) * 0x14));
  }
  return;
}

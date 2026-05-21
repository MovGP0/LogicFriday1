/* 00421516 FUN_00421516 */

undefined4 __thiscall FUN_00421516(void *this,int *param_1)

{
  void *pvVar1;
  int iVar2;
  int *piVar3;
  int *piVar4;
  int local_c;
  int local_8;
  
  *param_1 = *(int *)this;
  piVar3 = (int *)((int)this + 0xc4);
  piVar4 = param_1 + 0x31;
  for (iVar2 = 0x4b; iVar2 != 0; iVar2 = iVar2 + -1) {
    *piVar4 = *piVar3;
    piVar3 = piVar3 + 1;
    piVar4 = piVar4 + 1;
  }
  for (local_8 = 0; local_8 < *(int *)((int)this + 200); local_8 = local_8 + 1) {
    param_1[local_8 + 0x11] = 0;
    param_1[local_8 + 1] = 0;
    pvVar1 = _realloc((void *)param_1[local_8 + 0x21],*(int *)this << 2);
    param_1[local_8 + 0x21] = (int)pvVar1;
    for (local_c = 0; local_c < *(int *)this; local_c = local_c + 1) {
      if (*(int *)(*(int *)((int)this + local_8 * 4 + 0x84) + local_c * 4) == 0) {
        *(undefined4 *)(param_1[local_8 + 0x21] + local_c * 4) = 1;
        param_1[local_8 + 1] = param_1[local_8 + 1] + 1;
      }
      else if (*(int *)(*(int *)((int)this + local_8 * 4 + 0x84) + local_c * 4) == 2) {
        *(undefined4 *)(param_1[local_8 + 0x21] + local_c * 4) = 2;
        param_1[local_8 + 0x11] = param_1[local_8 + 0x11] + 1;
      }
      else {
        *(undefined4 *)(param_1[local_8 + 0x21] + local_c * 4) = 0;
      }
    }
  }
  return 0;
}

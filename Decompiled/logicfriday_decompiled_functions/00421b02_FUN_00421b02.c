/* 00421b02 FUN_00421b02 */

undefined4 __thiscall FUN_00421b02(void *this,uint *param_1)

{
  void *pvVar1;
  int iVar2;
  uint *puVar3;
  uint *puVar4;
  uint local_c;
  uint local_8;
  
  *(uint *)this = *param_1;
  puVar3 = param_1 + 0x31;
  puVar4 = (uint *)((int)this + 0xc4);
  for (iVar2 = 0x4b; iVar2 != 0; iVar2 = iVar2 + -1) {
    *puVar4 = *puVar3;
    puVar3 = puVar3 + 1;
    puVar4 = puVar4 + 1;
  }
  for (local_8 = 0; local_8 < param_1[0x32]; local_8 = local_8 + 1) {
    pvVar1 = _realloc(*(void **)((int)this + local_8 * 4 + 0x84),*param_1 << 2);
    *(void **)((int)this + local_8 * 4 + 0x84) = pvVar1;
    *(undefined4 *)((int)this + local_8 * 4 + 0x44) = 0;
    *(undefined4 *)((int)this + local_8 * 4 + 4) = 0;
    for (local_c = 0; local_c < *param_1; local_c = local_c + 1) {
      *(undefined4 *)(*(int *)((int)this + local_8 * 4 + 0x84) + local_c * 4) =
           *(undefined4 *)(param_1[local_8 + 0x21] + local_c * 4);
      if (*(int *)(*(int *)((int)this + local_8 * 4 + 0x84) + local_c * 4) == 1) {
        *(int *)((int)this + local_8 * 4 + 4) = *(int *)((int)this + local_8 * 4 + 4) + 1;
      }
      else if (*(int *)(*(int *)((int)this + local_8 * 4 + 0x84) + local_c * 4) == 2) {
        *(int *)((int)this + local_8 * 4 + 0x44) = *(int *)((int)this + local_8 * 4 + 0x44) + 1;
      }
    }
  }
  return 0;
}

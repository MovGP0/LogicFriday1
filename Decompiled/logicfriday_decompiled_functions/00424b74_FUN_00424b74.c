/* 00424b74 FUN_00424b74 */

undefined4 __fastcall FUN_00424b74(void *param_1)

{
  void *pvVar1;
  undefined4 uVar2;
  int iVar3;
  int local_c;
  int local_8;
  
  pvVar1 = _malloc(*(int *)((int)param_1 + 0x2670) << 2);
  *(void **)((int)param_1 + 0x2678) = pvVar1;
  if (*(int *)((int)param_1 + 0x2678) == 0) {
    uVar2 = 0x40016;
  }
  else {
    for (local_8 = 0; local_8 < *(int *)((int)param_1 + 0x2670); local_8 = local_8 + 1) {
      pvVar1 = _malloc(*(int *)((int)param_1 + 0x2674) * 0x48);
      *(void **)(*(int *)((int)param_1 + 0x2678) + local_8 * 4) = pvVar1;
      if (*(int *)(*(int *)((int)param_1 + 0x2678) + local_8 * 4) == 0) {
        return 0x40017;
      }
      _memset(*(void **)(*(int *)((int)param_1 + 0x2678) + local_8 * 4),0,
              *(int *)((int)param_1 + 0x2674) * 0x48);
      for (local_c = 0; local_c < *(int *)((int)param_1 + 0x2674); local_c = local_c + 1) {
        *(undefined4 *)(local_c * 0x48 + *(int *)(*(int *)((int)param_1 + 0x2678) + local_8 * 4)) =
             0xffffffff;
      }
    }
    for (local_8 = 0; local_8 < *(int *)((int)param_1 + 0x1654); local_8 = local_8 + 1) {
      **(int **)(*(int *)((int)param_1 + 0x2678) + local_8 * 4) = local_8;
      *(undefined4 *)(*(int *)(*(int *)((int)param_1 + 0x2678) + local_8 * 4) + 4) = 0;
    }
    for (local_8 = 1; local_8 < *(int *)((int)param_1 + 0x2674); local_8 = local_8 + 1) {
      iVar3 = FUN_00424ceb(param_1,local_8);
      if (iVar3 == 0) {
        return 0x40017;
      }
    }
    uVar2 = 0;
  }
  return uVar2;
}

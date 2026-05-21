/* 0042aca7 FUN_0042aca7 */

undefined4 __thiscall FUN_0042aca7(void *this,int param_1)

{
  int iVar1;
  int iVar2;
  int iVar3;
  int local_20;
  undefined4 local_c;
  int local_8;
  
  local_c = 0;
  for (local_8 = 0; local_8 < *(int *)((int)this + 0x16c4); local_8 = local_8 + 1) {
    if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0x48) == 0) {
      for (local_20 = 0;
          local_20 < *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0x18);
          local_20 = local_20 + 1) {
        if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xe4 + local_20 * 4) ==
            -3) {
          iVar1 = *(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4);
          iVar2 = *(int *)(iVar1 + 0x6c + local_20 * 8);
          iVar1 = *(int *)(iVar1 + 0x70 + local_20 * 8);
          iVar3 = FUN_0043bfa3(*(void **)(*(int *)((int)this + 0x16d0) + param_1 * 4),iVar2,iVar1);
          if (iVar3 != 0) {
            *(undefined4 *)((int)this + 0x24ec) = 0;
            *(int *)((int)this + 0x24f0) = local_8;
            *(int *)((int)this + 0x24f4) = local_20;
            *(undefined4 *)((int)this + 0x2500) = 2;
            *(int *)((int)this + 0x250c) = param_1;
            FUN_0043ac51((void *)((int)this + 0x24ec),iVar2,iVar1);
            FUN_0043ac51((void *)((int)this + 0x24ec),iVar2,iVar1);
            *(undefined4 *)((int)this + 0x2524) =
                 *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + param_1 * 4) + 0x38);
            FUN_00429b01();
            FUN_0043aba6((undefined4 *)((int)this + 0x24ec));
            *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xe4 + local_20 * 4) =
                 *(int *)((int)this + 0x16c8) + -1;
            local_c = 1;
          }
        }
      }
      if ((*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xe0) == -3) &&
         (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + param_1 * 4) + 0x38) == -3)) {
        iVar1 = *(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4);
        iVar2 = *(int *)(iVar1 + 0xac);
        iVar1 = *(int *)(iVar1 + 0xb0);
        iVar3 = FUN_0043bfa3(*(void **)(*(int *)((int)this + 0x16d0) + param_1 * 4),iVar2,iVar1);
        if (iVar3 != 0) {
          *(undefined4 *)((int)this + 0x24ec) = 1;
          *(int *)((int)this + 0x24f0) = local_8;
          *(undefined4 *)((int)this + 0x2500) = 2;
          *(int *)((int)this + 0x250c) = param_1;
          FUN_0043ac51((void *)((int)this + 0x24ec),iVar2,iVar1);
          FUN_0043ac51((void *)((int)this + 0x24ec),iVar2,iVar1);
          *(int *)((int)this + 0x2524) = local_8;
          FUN_00429b01();
          FUN_0043aba6((undefined4 *)((int)this + 0x24ec));
          *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xe0) =
               *(int *)((int)this + 0x16c8) + -1;
          local_c = 1;
        }
      }
    }
  }
  return local_c;
}

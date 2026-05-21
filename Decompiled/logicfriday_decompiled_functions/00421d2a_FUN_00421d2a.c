/* 00421d2a FUN_00421d2a */

undefined4 __thiscall FUN_00421d2a(void *this,int param_1,int param_2)

{
  uint uVar1;
  uint uVar2;
  char cVar3;
  char cVar4;
  uint local_14;
  uint local_c;
  
  uVar1 = *(uint *)((int)this + 0xc4);
  uVar2 = *(uint *)((int)this + 200);
  for (local_c = 0; local_c < uVar1; local_c = local_c + 1) {
    cVar3 = (char)uVar1;
    cVar4 = (char)local_c;
    if (*(char *)(param_2 + local_c) == '1') {
      *(uint *)(*(int *)((int)this + 0x1f8) + 4 + param_1 * 0xc) =
           *(uint *)(*(int *)((int)this + 0x1f8) + 4 + param_1 * 0xc) |
           1 << ((cVar3 - cVar4) - 1U & 0x1f);
    }
    else if (*(char *)(param_2 + local_c) == '-') {
      *(uint *)(*(int *)((int)this + 0x1f8) + 4 + param_1 * 0xc) =
           *(uint *)(*(int *)((int)this + 0x1f8) + 4 + param_1 * 0xc) |
           1 << ((cVar3 - cVar4) - 1U & 0x1f);
      *(uint *)(*(int *)((int)this + 0x1f8) + 8 + param_1 * 0xc) =
           *(uint *)(*(int *)((int)this + 0x1f8) + 8 + param_1 * 0xc) |
           1 << ((cVar3 - cVar4) - 1U & 0x1f);
      *(int *)(param_1 * 0xc + *(int *)((int)this + 0x1f8)) =
           *(int *)(*(int *)((int)this + 0x1f8) + param_1 * 0xc) + 1;
    }
  }
  for (local_14 = 0; local_14 < uVar2; local_14 = local_14 + 1) {
    if (*(char *)(param_2 + local_c + 1 + local_14) == '1') {
      *(undefined4 *)(*(int *)((int)this + local_14 * 4 + 0x1fc) + param_1 * 4) = 1;
    }
    else {
      *(undefined4 *)(*(int *)((int)this + local_14 * 4 + 0x1fc) + param_1 * 4) = 0;
    }
  }
  return 0;
}

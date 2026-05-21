/* 004161ea FUN_004161ea */

undefined4 __thiscall FUN_004161ea(void *this,int param_1)

{
  int iVar1;
  char cVar2;
  void *pvVar3;
  undefined4 uVar4;
  int iVar5;
  int local_64;
  int aiStack_54 [16];
  uint local_14;
  uint local_10;
  uint local_c;
  int local_8;
  
  for (local_10 = 0; (int)local_10 < 0x10; local_10 = local_10 + 1) {
    if (*(int *)((int)this + local_10 * 4 + 0x94) != 0) {
      _free(*(void **)((int)this + local_10 * 4 + 0x94));
      *(undefined4 *)((int)this + local_10 * 4 + 0x94) = 0;
    }
  }
  _memcpy((void *)((int)this + 0x10),*(void **)((int)this + 0xc),0x1f0);
  for (local_10 = 0; (int)local_10 < 0x10; local_10 = local_10 + 1) {
    *(undefined4 *)((int)this + local_10 * 4 + 0x94) = 0;
  }
  pvVar3 = _malloc(**(int **)((int)this + 0xc) << 2);
  *(void **)((int)this + 0x94) = pvVar3;
  if (*(int *)((int)this + 0x94) == 0) {
    uVar4 = 0x40010;
  }
  else {
    _memset(*(void **)((int)this + 0x94),0,**(int **)((int)this + 0xc) << 2);
    if (param_1 == 0) {
      _memcpy(*(void **)((int)this + 0x94),*(void **)(*(int *)((int)this + 0xc) + 0x84),
              **(int **)((int)this + 0xc) << 2);
      uVar4 = 0;
    }
    else {
      iVar1 = *(int *)((int)this + 0xd4);
      for (local_10 = 0; cVar2 = (char)iVar1, (int)local_10 < iVar1; local_10 = local_10 + 1) {
        for (local_64 = 0; local_64 < iVar1; local_64 = local_64 + 1) {
          iVar5 = _strcmp((char *)(*(int *)((int)this + 8) + 0x160 + local_10 * 9),
                          (char *)(*(int *)((int)this + 0xc) + 0x160 + local_64 * 9));
          if (iVar5 == 0) {
            local_8 = 1 << ((cVar2 + -1) - (char)local_10 & 0x1fU);
            aiStack_54[local_10] = (1 << ((cVar2 + -1) - (char)local_64 & 0x1fU)) - local_8;
            break;
          }
        }
      }
      iVar5 = *(int *)((int)this + 0x10);
      for (local_10 = 0; (int)local_10 < iVar5; local_10 = local_10 + 1) {
        local_14 = local_10;
        for (local_64 = 0; local_64 < iVar1; local_64 = local_64 + 1) {
          local_c = 1 << ((cVar2 + -1) - (char)local_64 & 0x1fU);
          if ((local_10 & local_c) != 0) {
            local_14 = local_14 + aiStack_54[local_64];
          }
        }
        *(undefined4 *)(*(int *)((int)this + 0x94) + local_10 * 4) =
             *(undefined4 *)(*(int *)(*(int *)((int)this + 0xc) + 0x84) + local_14 * 4);
      }
      uVar4 = 0;
    }
  }
  return uVar4;
}

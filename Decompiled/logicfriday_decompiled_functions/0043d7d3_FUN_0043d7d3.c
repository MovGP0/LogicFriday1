/* 0043d7d3 FUN_0043d7d3 */

void __thiscall FUN_0043d7d3(void *this,HWND param_1)

{
  undefined4 uVar1;
  void *pvVar2;
  int iVar3;
  int iVar4;
  undefined4 *puVar5;
  undefined4 *puVar6;
  int local_20;
  int *local_1c;
  int local_18;
  undefined4 local_14;
  void *local_10;
  int local_c;
  int local_8;
  
  local_10 = (void *)0x0;
  *(undefined4 *)((int)this + 0x3c) = 1;
  *(undefined4 *)((int)this + 0x44) = 0;
  if (*(int *)(*(int *)((int)this + 0x2c) + 8) == 0) {
    for (local_c = 0;
        (local_c < *(int *)((int)this + 0x28) &&
        (*(int *)(*(int *)((int)this + 0x2c) + 8 + local_c * 0x14) == 0)); local_c = local_c + 1) {
    }
    *(int *)((int)this + 0x28) = local_c + 1;
    pvVar2 = _realloc(*(void **)((int)this + 0x2c),*(int *)((int)this + 0x28) * 0x14);
    *(void **)((int)this + 0x2c) = pvVar2;
    if (*(int *)((int)this + 0x14) == 2) {
      SendMessageA(param_1,0x800d,(WPARAM)&local_1c,*(LPARAM *)((int)this + 0x20));
      FUN_0043af87(local_1c,*(int *)((int)this + 0x4c));
    }
    *(undefined4 *)((int)this + 0x14) = 0xfffffffd;
    if (*(int *)this != 1) {
      *(undefined4 *)((int)this + 0x38) = 0xfffffffd;
    }
  }
  else {
    for (local_c = *(int *)((int)this + 0x28) + -2;
        (-1 < local_c && (*(int *)(*(int *)((int)this + 0x2c) + 8 + local_c * 0x14) == 0));
        local_c = local_c + -1) {
    }
    local_20 = 0;
    local_8 = local_c + 1;
    for (local_c = local_8; local_c < *(int *)((int)this + 0x28); local_c = local_c + 1) {
      puVar5 = (undefined4 *)(*(int *)((int)this + 0x2c) + local_c * 0x14);
      puVar6 = (undefined4 *)(*(int *)((int)this + 0x2c) + local_20 * 0x14);
      for (iVar3 = 5; iVar3 != 0; iVar3 = iVar3 + -1) {
        *puVar6 = *puVar5;
        puVar5 = puVar5 + 1;
        puVar6 = puVar6 + 1;
      }
      local_20 = local_20 + 1;
    }
    *(int *)((int)this + 0x28) = local_20;
    pvVar2 = _realloc(*(void **)((int)this + 0x2c),*(int *)((int)this + 0x28) * 0x14);
    *(void **)((int)this + 0x2c) = pvVar2;
    if (*(int *)this == 2) {
      SendMessageA(param_1,0x800d,(WPARAM)&local_1c,*(LPARAM *)((int)this + 0xc));
      FUN_0043af87(local_1c,*(int *)((int)this + 0x4c));
    }
    *(undefined4 *)this = 0xfffffffd;
    if (*(int *)((int)this + 0x14) != 1) {
      *(undefined4 *)((int)this + 0x38) = 0xfffffffd;
    }
  }
  local_18 = 0;
  local_c = 0;
  do {
    if (*(int *)((int)this + 0x30) <= local_c) {
      pvVar2 = _realloc(*(void **)((int)this + 0x34),local_18 * 0x14);
      *(void **)((int)this + 0x34) = pvVar2;
      for (local_c = 0; local_c < local_18; local_c = local_c + 1) {
        puVar5 = (undefined4 *)((int)local_10 + local_c * 0x14);
        puVar6 = (undefined4 *)(*(int *)((int)this + 0x34) + local_c * 0x14);
        for (iVar3 = 5; iVar3 != 0; iVar3 = iVar3 + -1) {
          *puVar6 = *puVar5;
          puVar5 = puVar5 + 1;
          puVar6 = puVar6 + 1;
        }
      }
      _free(local_10);
      *(int *)((int)this + 0x30) = local_18;
      local_14 = 0;
      local_c = 0;
      while( true ) {
        if (*(int *)((int)this + 0x30) <= local_c) {
          return;
        }
        SendMessageA(param_1,0x800d,(WPARAM)&local_1c,
                     *(LPARAM *)(*(int *)((int)this + 0x34) + 0xc + local_c * 0x14));
        iVar3 = FUN_0043c96e(this,(int)local_1c);
        if (iVar3 != 0) break;
        local_c = local_c + 1;
      }
      if (*(int *)this == -3) {
        uVar1 = *(undefined4 *)(*(int *)((int)this + 0x34) + 4 + local_c * 0x14);
        puVar5 = *(undefined4 **)((int)this + 0x2c);
        *puVar5 = *(undefined4 *)(*(int *)((int)this + 0x34) + local_c * 0x14);
        puVar5[1] = uVar1;
      }
      else {
        uVar1 = *(undefined4 *)(*(int *)((int)this + 0x34) + 4 + local_c * 0x14);
        iVar4 = (*(int *)((int)this + 0x28) + -1) * 0x14;
        iVar3 = *(int *)((int)this + 0x2c);
        *(undefined4 *)(iVar3 + iVar4) =
             *(undefined4 *)(*(int *)((int)this + 0x34) + local_c * 0x14);
        *(undefined4 *)(iVar3 + 4 + iVar4) = uVar1;
      }
      FUN_0043cc09(this,local_1c,param_1);
      return;
    }
    local_20 = 0;
LAB_0043d9c6:
    if (*(int *)((int)this + 0x28) <= local_20) goto LAB_0043d9a3;
    if (*(int *)(*(int *)((int)this + 0x34) + 8 + local_c * 0x14) !=
        *(int *)(*(int *)((int)this + 0x2c) + 0x10 + local_20 * 0x14)) {
      if (((local_20 == 0) &&
          (*(int *)(local_c * 0x14 + *(int *)((int)this + 0x34)) == **(int **)((int)this + 0x2c)))
         && (*(int *)(*(int *)((int)this + 0x34) + 4 + local_c * 0x14) ==
             *(int *)(*(int *)((int)this + 0x2c) + 4))) {
        local_18 = local_18 + 1;
        local_10 = _realloc(local_10,local_18 * 0x14);
        puVar5 = (undefined4 *)(*(int *)((int)this + 0x34) + local_c * 0x14);
        puVar6 = (undefined4 *)((int)local_10 + (local_18 + -1) * 0x14);
        for (iVar3 = 5; iVar3 != 0; iVar3 = iVar3 + -1) {
          *puVar6 = *puVar5;
          puVar5 = puVar5 + 1;
          puVar6 = puVar6 + 1;
        }
        *(undefined4 *)(*(int *)((int)this + 0x34) + 8 + local_c * 0x14) =
             *(undefined4 *)(*(int *)((int)this + 0x2c) + 0x10);
      }
LAB_0043d9bf:
      local_20 = local_20 + 1;
      goto LAB_0043d9c6;
    }
    if ((local_20 == *(int *)((int)this + 0x28) + -1) &&
       ((*(int *)(local_c * 0x14 + *(int *)((int)this + 0x34)) !=
         *(int *)(local_20 * 0x14 + *(int *)((int)this + 0x2c)) ||
        (*(int *)(*(int *)((int)this + 0x34) + 4 + local_c * 0x14) !=
         *(int *)(*(int *)((int)this + 0x2c) + 4 + local_20 * 0x14))))) goto LAB_0043d9bf;
    local_18 = local_18 + 1;
    local_10 = _realloc(local_10,local_18 * 0x14);
    puVar5 = (undefined4 *)(*(int *)((int)this + 0x34) + local_c * 0x14);
    puVar6 = (undefined4 *)((int)local_10 + (local_18 + -1) * 0x14);
    for (iVar3 = 5; iVar3 != 0; iVar3 = iVar3 + -1) {
      *puVar6 = *puVar5;
      puVar5 = puVar5 + 1;
      puVar6 = puVar6 + 1;
    }
LAB_0043d9a3:
    local_c = local_c + 1;
  } while( true );
}

/* 0043cc09 FUN_0043cc09 */

undefined4 __thiscall FUN_0043cc09(void *this,int *param_1,HWND param_2)

{
  int iVar1;
  void *pvVar2;
  undefined4 *puVar3;
  undefined4 *puVar4;
  undefined4 local_30 [5];
  int local_1c;
  int local_18;
  int local_14;
  int local_10;
  int local_c;
  int local_8;
  
  if (((*(int *)((int)this + 0x38) != -3) && (param_1[0xe] != -3)) &&
     (*(int *)((int)this + 0x38) != param_1[0xe])) {
    return 0;
  }
  if (*(int *)((int)this + 0x14) == -3) {
    iVar1 = (*(int *)((int)this + 0x28) + -1) * 0x14;
    local_18 = *(int *)(*(int *)((int)this + 0x2c) + iVar1);
    local_14 = *(int *)(*(int *)((int)this + 0x2c) + 4 + iVar1);
    local_8 = param_1[10] + -1;
    if ((local_18 == *(int *)param_1[0xb]) && (local_14 == *(int *)(param_1[0xb] + 4))) {
      local_10 = *(int *)((int)this + 0x28) + -1 + param_1[10];
      pvVar2 = _realloc(*(void **)((int)this + 0x2c),local_10 * 0x14);
      *(void **)((int)this + 0x2c) = pvVar2;
      local_1c = *(int *)((int)this + 0x28);
      for (local_c = 1; local_c < param_1[10]; local_c = local_c + 1) {
        puVar3 = (undefined4 *)(param_1[0xb] + local_c * 0x14);
        puVar4 = (undefined4 *)(*(int *)((int)this + 0x2c) + local_1c * 0x14);
        for (iVar1 = 5; iVar1 != 0; iVar1 = iVar1 + -1) {
          *puVar4 = *puVar3;
          puVar3 = puVar3 + 1;
          puVar4 = puVar4 + 1;
        }
        local_1c = local_1c + 1;
      }
      *(int *)((int)this + 0x28) = local_10;
      local_10 = *(int *)((int)this + 0x30) + param_1[0xc];
      pvVar2 = _realloc(*(void **)((int)this + 0x34),local_10 * 0x14);
      *(void **)((int)this + 0x34) = pvVar2;
      local_1c = *(int *)((int)this + 0x30);
      for (local_c = 0; local_c < param_1[0xc]; local_c = local_c + 1) {
        puVar3 = (undefined4 *)(param_1[0xd] + local_c * 0x14);
        puVar4 = (undefined4 *)(*(int *)((int)this + 0x34) + local_1c * 0x14);
        for (iVar1 = 5; iVar1 != 0; iVar1 = iVar1 + -1) {
          *puVar4 = *puVar3;
          puVar3 = puVar3 + 1;
          puVar4 = puVar4 + 1;
        }
        local_1c = local_1c + 1;
      }
      *(int *)((int)this + 0x30) = local_10;
      *(int *)((int)this + 0x14) = param_1[5];
      *(int *)((int)this + 0x18) = param_1[6];
      *(int *)((int)this + 0x1c) = param_1[7];
      *(int *)((int)this + 0x20) = param_1[8];
      if (*(int *)this == 1) {
        *(undefined4 *)((int)this + 0x38) = *(undefined4 *)((int)this + 4);
      }
      else if (*(int *)((int)this + 0x14) == 1) {
        *(undefined4 *)((int)this + 0x38) = *(undefined4 *)((int)this + 0x18);
      }
      else {
        *(undefined4 *)((int)this + 0x38) = 0xfffffffd;
      }
      param_1[0x10] = 1;
      *(undefined4 *)((int)this + 0x3c) = 0;
      FUN_0043d4ba(this,param_2);
      return 1;
    }
    if ((local_18 == *(int *)(local_8 * 0x14 + param_1[0xb])) &&
       (local_14 == *(int *)(param_1[0xb] + 4 + local_8 * 0x14))) {
      local_10 = *(int *)((int)this + 0x28) + -1 + param_1[10];
      pvVar2 = _realloc(*(void **)((int)this + 0x2c),local_10 * 0x14);
      *(void **)((int)this + 0x2c) = pvVar2;
      local_1c = *(int *)((int)this + 0x28);
      for (local_c = param_1[10] + -2; -1 < local_c; local_c = local_c + -1) {
        puVar3 = (undefined4 *)(param_1[0xb] + local_c * 0x14);
        puVar4 = (undefined4 *)(*(int *)((int)this + 0x2c) + local_1c * 0x14);
        for (iVar1 = 5; iVar1 != 0; iVar1 = iVar1 + -1) {
          *puVar4 = *puVar3;
          puVar3 = puVar3 + 1;
          puVar4 = puVar4 + 1;
        }
        local_1c = local_1c + 1;
      }
      *(int *)((int)this + 0x28) = local_10;
      local_10 = *(int *)((int)this + 0x30) + param_1[0xc];
      pvVar2 = _realloc(*(void **)((int)this + 0x34),local_10 * 0x14);
      *(void **)((int)this + 0x34) = pvVar2;
      local_1c = *(int *)((int)this + 0x30);
      for (local_c = 0; local_c < param_1[0xc]; local_c = local_c + 1) {
        puVar3 = (undefined4 *)(param_1[0xd] + local_c * 0x14);
        puVar4 = (undefined4 *)(*(int *)((int)this + 0x34) + local_1c * 0x14);
        for (iVar1 = 5; iVar1 != 0; iVar1 = iVar1 + -1) {
          *puVar4 = *puVar3;
          puVar3 = puVar3 + 1;
          puVar4 = puVar4 + 1;
        }
        local_1c = local_1c + 1;
      }
      *(int *)((int)this + 0x30) = local_10;
      *(int *)((int)this + 0x14) = *param_1;
      *(int *)((int)this + 0x18) = param_1[1];
      *(int *)((int)this + 0x1c) = param_1[2];
      *(int *)((int)this + 0x20) = param_1[3];
      if (*(int *)this == 1) {
        *(undefined4 *)((int)this + 0x38) = *(undefined4 *)((int)this + 4);
      }
      else if (*(int *)((int)this + 0x14) == 1) {
        *(undefined4 *)((int)this + 0x38) = *(undefined4 *)((int)this + 0x18);
      }
      else {
        *(undefined4 *)((int)this + 0x38) = 0xfffffffd;
      }
      param_1[0x10] = 1;
      *(undefined4 *)((int)this + 0x3c) = 0;
      FUN_0043d4ba(this,param_2);
      return 1;
    }
  }
  else {
    local_18 = **(int **)((int)this + 0x2c);
    local_14 = (*(int **)((int)this + 0x2c))[1];
    local_8 = param_1[10] + -1;
    if ((local_18 == *(int *)param_1[0xb]) && (local_14 == *(int *)(param_1[0xb] + 4))) {
      for (local_c = 0; local_c < *(int *)((int)this + 0x28) / 2; local_c = local_c + 1) {
        puVar3 = (undefined4 *)(*(int *)((int)this + 0x2c) + local_c * 0x14);
        puVar4 = local_30;
        for (iVar1 = 5; iVar1 != 0; iVar1 = iVar1 + -1) {
          *puVar4 = *puVar3;
          puVar3 = puVar3 + 1;
          puVar4 = puVar4 + 1;
        }
        puVar3 = (undefined4 *)
                 (*(int *)((int)this + 0x2c) + ((*(int *)((int)this + 0x28) + -1) - local_c) * 0x14)
        ;
        puVar4 = (undefined4 *)(*(int *)((int)this + 0x2c) + local_c * 0x14);
        for (iVar1 = 5; iVar1 != 0; iVar1 = iVar1 + -1) {
          *puVar4 = *puVar3;
          puVar3 = puVar3 + 1;
          puVar4 = puVar4 + 1;
        }
        puVar3 = local_30;
        puVar4 = (undefined4 *)
                 (*(int *)((int)this + 0x2c) + ((*(int *)((int)this + 0x28) + -1) - local_c) * 0x14)
        ;
        for (iVar1 = 5; iVar1 != 0; iVar1 = iVar1 + -1) {
          *puVar4 = *puVar3;
          puVar3 = puVar3 + 1;
          puVar4 = puVar4 + 1;
        }
      }
      *(undefined4 *)this = *(undefined4 *)((int)this + 0x14);
      *(undefined4 *)((int)this + 4) = *(undefined4 *)((int)this + 0x18);
      *(undefined4 *)((int)this + 8) = *(undefined4 *)((int)this + 0x1c);
      *(undefined4 *)((int)this + 0xc) = *(undefined4 *)((int)this + 0x20);
      local_10 = *(int *)((int)this + 0x28) + -1 + param_1[10];
      pvVar2 = _realloc(*(void **)((int)this + 0x2c),local_10 * 0x14);
      *(void **)((int)this + 0x2c) = pvVar2;
      local_1c = *(int *)((int)this + 0x28);
      for (local_c = 1; local_c < param_1[10]; local_c = local_c + 1) {
        puVar3 = (undefined4 *)(param_1[0xb] + local_c * 0x14);
        puVar4 = (undefined4 *)(*(int *)((int)this + 0x2c) + local_1c * 0x14);
        for (iVar1 = 5; iVar1 != 0; iVar1 = iVar1 + -1) {
          *puVar4 = *puVar3;
          puVar3 = puVar3 + 1;
          puVar4 = puVar4 + 1;
        }
        local_1c = local_1c + 1;
      }
      *(int *)((int)this + 0x28) = local_10;
      local_10 = *(int *)((int)this + 0x30) + param_1[0xc];
      pvVar2 = _realloc(*(void **)((int)this + 0x34),local_10 * 0x14);
      *(void **)((int)this + 0x34) = pvVar2;
      local_1c = *(int *)((int)this + 0x30);
      for (local_c = 0; local_c < param_1[0xc]; local_c = local_c + 1) {
        puVar3 = (undefined4 *)(param_1[0xd] + local_c * 0x14);
        puVar4 = (undefined4 *)(*(int *)((int)this + 0x34) + local_1c * 0x14);
        for (iVar1 = 5; iVar1 != 0; iVar1 = iVar1 + -1) {
          *puVar4 = *puVar3;
          puVar3 = puVar3 + 1;
          puVar4 = puVar4 + 1;
        }
        local_1c = local_1c + 1;
      }
      *(int *)((int)this + 0x30) = local_10;
      *(int *)((int)this + 0x14) = param_1[5];
      *(int *)((int)this + 0x18) = param_1[6];
      *(int *)((int)this + 0x1c) = param_1[7];
      *(int *)((int)this + 0x20) = param_1[8];
      if (*(int *)this == 1) {
        *(undefined4 *)((int)this + 0x38) = *(undefined4 *)((int)this + 4);
      }
      else if (*(int *)((int)this + 0x14) == 1) {
        *(undefined4 *)((int)this + 0x38) = *(undefined4 *)((int)this + 0x18);
      }
      else {
        *(undefined4 *)((int)this + 0x38) = 0xfffffffd;
      }
      param_1[0x10] = 1;
      *(undefined4 *)((int)this + 0x3c) = 0;
      FUN_0043d4ba(this,param_2);
      return 1;
    }
    if ((local_18 == *(int *)(local_8 * 0x14 + param_1[0xb])) &&
       (local_14 == *(int *)(param_1[0xb] + 4 + local_8 * 0x14))) {
      for (local_c = 0; local_c < *(int *)((int)this + 0x28) / 2; local_c = local_c + 1) {
        puVar3 = (undefined4 *)(*(int *)((int)this + 0x2c) + local_c * 0x14);
        puVar4 = local_30;
        for (iVar1 = 5; iVar1 != 0; iVar1 = iVar1 + -1) {
          *puVar4 = *puVar3;
          puVar3 = puVar3 + 1;
          puVar4 = puVar4 + 1;
        }
        puVar3 = (undefined4 *)
                 (*(int *)((int)this + 0x2c) + ((*(int *)((int)this + 0x28) + -1) - local_c) * 0x14)
        ;
        puVar4 = (undefined4 *)(*(int *)((int)this + 0x2c) + local_c * 0x14);
        for (iVar1 = 5; iVar1 != 0; iVar1 = iVar1 + -1) {
          *puVar4 = *puVar3;
          puVar3 = puVar3 + 1;
          puVar4 = puVar4 + 1;
        }
        puVar3 = local_30;
        puVar4 = (undefined4 *)
                 (*(int *)((int)this + 0x2c) + ((*(int *)((int)this + 0x28) + -1) - local_c) * 0x14)
        ;
        for (iVar1 = 5; iVar1 != 0; iVar1 = iVar1 + -1) {
          *puVar4 = *puVar3;
          puVar3 = puVar3 + 1;
          puVar4 = puVar4 + 1;
        }
      }
      *(undefined4 *)this = *(undefined4 *)((int)this + 0x14);
      *(undefined4 *)((int)this + 4) = *(undefined4 *)((int)this + 0x18);
      *(undefined4 *)((int)this + 8) = *(undefined4 *)((int)this + 0x1c);
      *(undefined4 *)((int)this + 0xc) = *(undefined4 *)((int)this + 0x20);
      local_10 = *(int *)((int)this + 0x28) + -1 + param_1[10];
      pvVar2 = _realloc(*(void **)((int)this + 0x2c),local_10 * 0x14);
      *(void **)((int)this + 0x2c) = pvVar2;
      local_1c = *(int *)((int)this + 0x28);
      for (local_c = param_1[10] + -2; -1 < local_c; local_c = local_c + -1) {
        puVar3 = (undefined4 *)(param_1[0xb] + local_c * 0x14);
        puVar4 = (undefined4 *)(*(int *)((int)this + 0x2c) + local_1c * 0x14);
        for (iVar1 = 5; iVar1 != 0; iVar1 = iVar1 + -1) {
          *puVar4 = *puVar3;
          puVar3 = puVar3 + 1;
          puVar4 = puVar4 + 1;
        }
        local_1c = local_1c + 1;
      }
      *(int *)((int)this + 0x28) = local_10;
      local_10 = *(int *)((int)this + 0x30) + param_1[0xc];
      pvVar2 = _realloc(*(void **)((int)this + 0x34),local_10 * 0x14);
      *(void **)((int)this + 0x34) = pvVar2;
      local_1c = *(int *)((int)this + 0x30);
      for (local_c = 0; local_c < param_1[0xc]; local_c = local_c + 1) {
        puVar3 = (undefined4 *)(param_1[0xd] + local_c * 0x14);
        puVar4 = (undefined4 *)(*(int *)((int)this + 0x34) + local_1c * 0x14);
        for (iVar1 = 5; iVar1 != 0; iVar1 = iVar1 + -1) {
          *puVar4 = *puVar3;
          puVar3 = puVar3 + 1;
          puVar4 = puVar4 + 1;
        }
        local_1c = local_1c + 1;
      }
      *(int *)((int)this + 0x30) = local_10;
      *(int *)((int)this + 0x14) = *param_1;
      *(int *)((int)this + 0x18) = param_1[1];
      *(int *)((int)this + 0x1c) = param_1[2];
      *(int *)((int)this + 0x20) = param_1[3];
      if (*(int *)this == 1) {
        *(undefined4 *)((int)this + 0x38) = *(undefined4 *)((int)this + 4);
      }
      else if (*(int *)((int)this + 0x14) == 1) {
        *(undefined4 *)((int)this + 0x38) = *(undefined4 *)((int)this + 0x18);
      }
      else {
        *(undefined4 *)((int)this + 0x38) = 0xfffffffd;
      }
      param_1[0x10] = 1;
      *(undefined4 *)((int)this + 0x3c) = 0;
      FUN_0043d4ba(this,param_2);
      return 1;
    }
  }
  return 0;
}

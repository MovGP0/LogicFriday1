/* 0043dc5c FUN_0043dc5c */

void __thiscall FUN_0043dc5c(void *this,int *param_1,HWND param_2)

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
  int local_14;
  void *local_10;
  int local_c;
  int local_8;
  
  local_10 = (void *)0x0;
  *(undefined4 *)((int)this + 0x3c) = 1;
  *(undefined4 *)((int)this + 0x44) = 0;
  param_1[0xf] = 1;
  for (local_c = *(int *)((int)this + 0x28) + -2;
      (-1 < local_c && (*(int *)(*(int *)((int)this + 0x2c) + 8 + local_c * 0x14) == 0));
      local_c = local_c + -1) {
  }
  local_8 = local_c + 1;
  param_1[10] = *(int *)((int)this + 0x28) - local_8;
  pvVar2 = _malloc(param_1[10] * 0x14);
  param_1[0xb] = (int)pvVar2;
  local_20 = 0;
  for (local_c = local_8; local_c < *(int *)((int)this + 0x28); local_c = local_c + 1) {
    puVar5 = (undefined4 *)(*(int *)((int)this + 0x2c) + local_c * 0x14);
    puVar6 = (undefined4 *)(param_1[0xb] + local_20 * 0x14);
    for (iVar3 = 5; iVar3 != 0; iVar3 = iVar3 + -1) {
      *puVar6 = *puVar5;
      puVar5 = puVar5 + 1;
      puVar6 = puVar6 + 1;
    }
    local_20 = local_20 + 1;
  }
  local_18 = 0;
  for (local_c = 0; local_c < *(int *)((int)this + 0x30); local_c = local_c + 1) {
    for (local_20 = 0; local_20 < param_1[10]; local_20 = local_20 + 1) {
      if (*(int *)(*(int *)((int)this + 0x34) + 8 + local_c * 0x14) ==
          *(int *)(param_1[0xb] + 0x10 + local_20 * 0x14)) {
        local_18 = local_18 + 1;
        break;
      }
      if (((local_20 == 0) &&
          (*(int *)(local_c * 0x14 + *(int *)((int)this + 0x34)) == *(int *)param_1[0xb])) &&
         (*(int *)(*(int *)((int)this + 0x34) + 4 + local_c * 0x14) == *(int *)(param_1[0xb] + 4)))
      {
        local_18 = local_18 + 1;
      }
    }
  }
  if (local_18 != 0) {
    param_1[0xc] = local_18;
    pvVar2 = _malloc(param_1[0xc] * 0x14);
    param_1[0xd] = (int)pvVar2;
    local_14 = 0;
    for (local_c = 0; local_c < *(int *)((int)this + 0x30); local_c = local_c + 1) {
      local_20 = 0;
      while( true ) {
        if (param_1[10] <= local_20) goto LAB_0043de1a;
        if (*(int *)(*(int *)((int)this + 0x34) + 8 + local_c * 0x14) ==
            *(int *)(param_1[0xb] + 0x10 + local_20 * 0x14)) break;
        if (((local_20 == 0) &&
            (*(int *)(local_c * 0x14 + *(int *)((int)this + 0x34)) == *(int *)param_1[0xb])) &&
           (*(int *)(*(int *)((int)this + 0x34) + 4 + local_c * 0x14) == *(int *)(param_1[0xb] + 4))
           ) {
          puVar5 = (undefined4 *)(*(int *)((int)this + 0x34) + local_c * 0x14);
          puVar6 = (undefined4 *)(param_1[0xd] + local_14 * 0x14);
          for (iVar3 = 5; iVar3 != 0; iVar3 = iVar3 + -1) {
            *puVar6 = *puVar5;
            puVar5 = puVar5 + 1;
            puVar6 = puVar6 + 1;
          }
          *(undefined4 *)(param_1[0xd] + 8 + local_14 * 0x14) =
               *(undefined4 *)(*(int *)((int)this + 0x2c) + 0x10);
          local_14 = local_14 + 1;
        }
        local_20 = local_20 + 1;
      }
      puVar5 = (undefined4 *)(*(int *)((int)this + 0x34) + local_c * 0x14);
      puVar6 = (undefined4 *)(param_1[0xd] + local_14 * 0x14);
      for (iVar3 = 5; iVar3 != 0; iVar3 = iVar3 + -1) {
        *puVar6 = *puVar5;
        puVar5 = puVar5 + 1;
        puVar6 = puVar6 + 1;
      }
      local_14 = local_14 + 1;
LAB_0043de1a:
    }
  }
  param_1[5] = *(int *)((int)this + 0x14);
  param_1[6] = *(int *)((int)this + 0x18);
  param_1[7] = *(int *)((int)this + 0x1c);
  param_1[8] = *(int *)((int)this + 0x20);
  if (*(int *)((int)this + 0x14) == 1) {
    param_1[0xe] = *(int *)((int)this + 0x38);
  }
  else if (*(int *)((int)this + 0x14) == 2) {
    SendMessageA(param_2,0x800d,(WPARAM)&local_1c,*(LPARAM *)((int)this + 0x20));
    FUN_0043e2f5(local_1c,*(int *)((int)this + 0x4c),param_1[0x13]);
  }
  local_c = 0;
  do {
    if (param_1[0xc] <= local_c) {
LAB_0043e063:
      for (local_c = 0;
          (local_c < *(int *)((int)this + 0x28) &&
          (*(int *)(*(int *)((int)this + 0x2c) + 8 + local_c * 0x14) == 0)); local_c = local_c + 1)
      {
      }
      *(int *)((int)this + 0x28) = local_c + 1;
      pvVar2 = _realloc(*(void **)((int)this + 0x2c),*(int *)((int)this + 0x28) * 0x14);
      *(void **)((int)this + 0x2c) = pvVar2;
      *(undefined4 *)((int)this + 0x14) = 0xfffffffd;
      if (*(int *)this != 1) {
        *(undefined4 *)((int)this + 0x38) = 0xfffffffd;
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
          *(undefined4 *)((int)this + 0x14) = 0xfffffffd;
          local_c = 0;
          while( true ) {
            if (*(int *)((int)this + 0x30) <= local_c) {
              return;
            }
            SendMessageA(param_2,0x800d,(WPARAM)&local_1c,
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
          FUN_0043cc09(this,local_1c,param_2);
          return;
        }
        local_20 = 0;
        while( true ) {
          if (*(int *)((int)this + 0x28) <= local_20) goto LAB_0043e0df;
          if ((*(int *)(*(int *)((int)this + 0x34) + 8 + local_c * 0x14) ==
               *(int *)(*(int *)((int)this + 0x2c) + 0x10 + local_20 * 0x14)) &&
             ((local_20 != *(int *)((int)this + 0x28) + -1 ||
              ((*(int *)(local_c * 0x14 + *(int *)((int)this + 0x34)) ==
                *(int *)(local_20 * 0x14 + *(int *)((int)this + 0x2c)) &&
               (*(int *)(*(int *)((int)this + 0x34) + 4 + local_c * 0x14) ==
                *(int *)(*(int *)((int)this + 0x2c) + 4 + local_20 * 0x14))))))) break;
          local_20 = local_20 + 1;
        }
        local_18 = local_18 + 1;
        local_10 = _realloc(local_10,local_18 * 0x14);
        puVar5 = (undefined4 *)(*(int *)((int)this + 0x34) + local_c * 0x14);
        puVar6 = (undefined4 *)((int)local_10 + (local_18 + -1) * 0x14);
        for (iVar3 = 5; iVar3 != 0; iVar3 = iVar3 + -1) {
          *puVar6 = *puVar5;
          puVar5 = puVar5 + 1;
          puVar6 = puVar6 + 1;
        }
LAB_0043e0df:
        local_c = local_c + 1;
      } while( true );
    }
    SendMessageA(param_2,0x800d,(WPARAM)&local_1c,*(LPARAM *)(param_1[0xd] + 0xc + local_c * 0x14));
    iVar3 = FUN_0043c96e(param_1,(int)local_1c);
    if (iVar3 != 0) {
      if (*param_1 == -3) {
        uVar1 = *(undefined4 *)(param_1[0xd] + 4 + local_c * 0x14);
        puVar5 = (undefined4 *)param_1[0xb];
        *puVar5 = *(undefined4 *)(param_1[0xd] + local_c * 0x14);
        puVar5[1] = uVar1;
      }
      else {
        uVar1 = *(undefined4 *)(param_1[0xd] + 4 + local_c * 0x14);
        iVar4 = (*(int *)((int)this + 0x28) + -1) * 0x14;
        iVar3 = param_1[0xb];
        *(undefined4 *)(iVar3 + iVar4) = *(undefined4 *)(param_1[0xd] + local_c * 0x14);
        *(undefined4 *)(iVar3 + 4 + iVar4) = uVar1;
      }
      FUN_0043cc09(param_1,local_1c,param_2);
      goto LAB_0043e063;
    }
    local_c = local_c + 1;
  } while( true );
}

/* 00425b8e FUN_00425b8e */

void __thiscall FUN_00425b8e(void *this,HDC param_1)

{
  size_t c;
  tagSIZE *psizl;
  int local_3c;
  int local_38;
  int local_34;
  int local_30;
  tagSIZE local_28;
  int local_20;
  uint local_1c;
  int local_18;
  int local_14;
  int local_10;
  int local_c;
  int local_8;
  
  local_20 = 0;
  local_30 = 0;
  for (local_1c = 0; local_1c < *(uint *)((int)this + 0xc4); local_1c = local_1c + 1) {
    psizl = &local_28;
    c = _strlen((char *)((int)this + local_1c * 9 + 0x160));
    GetTextExtentPoint32A(param_1,(LPCSTR)((int)this + local_1c * 9 + 0x160),c,psizl);
    if (local_20 < local_28.cx) {
      local_20 = local_28.cx + 4;
    }
  }
  if (local_20 % 5 != 0) {
    local_20 = local_20 + (5 - local_20 % 5);
  }
  local_38 = local_28.cy + 0x32;
  local_10 = local_38 % 5;
  if (local_10 != 0) {
    local_38 = local_38 + (5 - local_10);
  }
  local_34 = local_20;
  SetPixel(param_1,0,0,0xffffff);
  for (local_18 = 0; local_18 < *(int *)((int)this + 0x2670); local_18 = local_18 + 1) {
    local_14 = 0;
    for (local_3c = 0; local_3c < *(int *)((int)this + 0x2674); local_3c = local_3c + 1) {
      if (*(int *)(*(int *)((int)this + 0x1668) + local_3c * 4) == 0) {
        local_34 = local_34 +
                   (*(int *)(**(int **)((int)this + 0x2678) + 8 + (local_3c + -1) * 0x48) + 1) * 0xf
        ;
        *(int *)(**(int **)((int)this + 0x2678) + 0x1c + local_3c * 0x48) = local_34 + 0x69;
      }
      else if (local_3c == 1) {
        local_34 = local_34 + 0x37 + *(int *)(**(int **)((int)this + 0x2678) + 8) * 0xf;
        *(int *)(**(int **)((int)this + 0x2678) + 100) = local_34;
      }
      else if (local_3c == *(int *)((int)this + 0x2674) + -1) {
        local_34 = local_34 + 0x69 +
                   *(int *)(**(int **)((int)this + 0x2678) + 8 + (local_3c + -1) * 0x48) * 0xf;
        *(int *)(**(int **)((int)this + 0x2678) + 0x1c + local_3c * 0x48) = local_34;
      }
      else if (local_3c != 0) {
        local_34 = local_34 + 0x69 +
                   *(int *)(**(int **)((int)this + 0x2678) + 8 + (local_3c + -1) * 0x48) * 0xf;
        *(int *)(**(int **)((int)this + 0x2678) + 0x1c + local_3c * 0x48) = local_34;
      }
      if (*(int *)(local_3c * 0x48 + *(int *)(*(int *)((int)this + 0x2678) + local_18 * 4)) != -1) {
        local_c = local_34;
        local_8 = local_30 +
                  *(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_18 * 4) + 4 +
                          local_3c * 0x48);
        FUN_00425f03(param_1,(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_18 * 4)
                                             + local_3c * 0x48) * 0xfc + *(int *)((int)this + 0x3a4)
                                    ),local_34,local_8,1);
        if (local_14 <
            *(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_18 * 4) + 4 + local_3c * 0x48)) {
          local_14 = *(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_18 * 4) + 4 +
                             local_3c * 0x48);
        }
      }
    }
    *(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_18 * 4) + 0x20) = local_30;
    *(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_18 * 4) + 0x24) =
         local_30 + local_38 + local_14;
    local_30 = (*(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_18 * 4) + 0x14) + 1) * 0xf +
               *(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_18 * 4) + 0x24);
    local_34 = local_20;
  }
  return;
}

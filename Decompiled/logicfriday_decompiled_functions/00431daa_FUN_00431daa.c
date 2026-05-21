/* 00431daa FUN_00431daa */

undefined4 __thiscall
FUN_00431daa(void *this,HDC param_1,int param_2,int param_3,int param_4,int param_5)

{
  HBRUSH pHVar1;
  HGDIOBJ h;
  int local_1c;
  int local_14;
  HGDIOBJ local_8;
  
  if (param_4 == 0) {
    pHVar1 = GetStockObject(0);
    FillRect(param_1,(RECT *)((int)this + 0x2338),pHVar1);
    *(undefined4 *)((int)this + 0x26ec) = 0xdf0000;
  }
  else {
    *(undefined4 *)((int)this + 0x26ec) = 0;
    if (param_5 == 0) {
      SetPixel(param_1,0,0,0xffffff);
    }
  }
  for (local_14 = 0; local_14 < *(int *)((int)this + 0x16c4); local_14 = local_14 + 1) {
    *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 0xe0) = 0xfffffffd;
    for (local_1c = 0;
        local_1c < *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 0x18);
        local_1c = local_1c + 1) {
      *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 0xe4 + local_1c * 4) =
           0xfffffffd;
    }
    if (((*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 0xdc) == 0) ||
        (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 0x48) != 0)) ||
       (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 0xd8) != 0)) {
      if ((*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 0x48) == 0) &&
         (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 0xd8) != 0)) {
        OffsetRect((LPRECT)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 200),param_2,
                   param_3);
        pHVar1 = GetStockObject(1);
        FillRect(param_1,(RECT *)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 200),
                 pHVar1);
      }
    }
    else {
      FUN_00425f03(param_1,*(int **)(*(int *)((int)this + 0x16cc) + local_14 * 4),
                   *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 0xc0) + param_2,
                   *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 0xc4) + param_3,
                   1);
    }
  }
  if (param_4 == 0) {
    local_8 = SelectObject(param_1,*(HGDIOBJ *)((int)this + 9000));
  }
  for (local_14 = 0; local_14 < *(int *)((int)this + 0x16c8); local_14 = local_14 + 1) {
    if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x40) == 0) {
      if ((param_2 != 0) || (param_3 != 0)) {
        for (local_1c = 0;
            local_1c < *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x28);
            local_1c = local_1c + 1) {
          *(int *)(local_1c * 0x14 +
                  *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x2c)) =
               *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x2c) +
                       local_1c * 0x14) + param_2;
          *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x2c) + 4 +
                  local_1c * 0x14) =
               *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x2c) + 4 +
                       local_1c * 0x14) + param_3;
        }
      }
      if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x44) != 0) {
        h = SelectObject(param_1,*(HGDIOBJ *)((int)this + 0x232c));
        for (local_1c = 0;
            local_1c < *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x28) + -1;
            local_1c = local_1c + 1) {
          if (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x2c) + 8 +
                      local_1c * 0x14) != 0) {
            MoveToEx(param_1,*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4)
                                              + 0x2c) + local_1c * 0x14),
                     *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x2c)
                              + 4 + local_1c * 0x14),(LPPOINT)0x0);
            LineTo(param_1,*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) +
                                            0x2c) + (local_1c + 1) * 0x14),
                   *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x2c) +
                            4 + (local_1c + 1) * 0x14));
          }
        }
        SelectObject(param_1,h);
      }
      MoveToEx(param_1,**(int **)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x2c),
               *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x2c) + 4),
               (LPPOINT)0x0);
      if (**(int **)(*(int *)((int)this + 0x16d0) + local_14 * 4) == 2) {
        FUN_004287c6(this,param_1,
                     **(int **)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x2c),
                     *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x2c)
                             + 4));
      }
      else if (**(int **)(*(int *)((int)this + 0x16d0) + local_14 * 4) == 1) {
        if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) +
                             *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 4) * 4
                             ) + 0xe0) == -3) {
          *(int *)(*(int *)(*(int *)((int)this + 0x16cc) +
                           *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 4) * 4)
                  + 0xe0) = local_14;
        }
        else {
          FUN_004287c6(this,param_1,
                       **(int **)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x2c),
                       *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) +
                                        0x2c) + 4));
        }
      }
      else if (**(int **)(*(int *)((int)this + 0x16d0) + local_14 * 4) == 0) {
        if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) +
                             *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 4) * 4
                             ) + 0xe4 +
                    *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 8) * 4) == -3)
        {
          *(int *)(*(int *)(*(int *)((int)this + 0x16cc) +
                           *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 4) * 4)
                   + 0xe4 + *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 8) * 4)
               = local_14;
        }
        else {
          FUN_004287c6(this,param_1,
                       **(int **)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x2c),
                       *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) +
                                        0x2c) + 4));
        }
      }
      for (local_1c = 1;
          local_1c < *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x28);
          local_1c = local_1c + 1) {
        LineTo(param_1,*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) +
                                        0x2c) + local_1c * 0x14),
               *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x2c) + 4 +
                       local_1c * 0x14));
      }
      if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x14) == 2) {
        FUN_004287c6(this,param_1,
                     *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x2c)
                             + (local_1c + -1) * 0x14),
                     *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x2c)
                              + 4 + (local_1c + -1) * 0x14));
      }
      else if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x14) == 1) {
        if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) +
                             *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x18)
                             * 4) + 0xe0) == -3) {
          *(int *)(*(int *)(*(int *)((int)this + 0x16cc) +
                           *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x18) *
                           4) + 0xe0) = local_14;
        }
        else {
          FUN_004287c6(this,param_1,
                       *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) +
                                        0x2c) + (local_1c + -1) * 0x14),
                       *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) +
                                        0x2c) + 4 + (local_1c + -1) * 0x14));
        }
      }
      else if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x14) == 0) {
        if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) +
                             *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x18)
                             * 4) + 0xe4 +
                    *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x1c) * 4) ==
            -3) {
          *(int *)(*(int *)(*(int *)((int)this + 0x16cc) +
                           *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x18) *
                           4) + 0xe4 +
                  *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) + 0x1c) * 4) =
               local_14;
        }
        else {
          FUN_004287c6(this,param_1,
                       *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) +
                                        0x2c) + (local_1c + -1) * 0x14),
                       *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_14 * 4) +
                                        0x2c) + 4 + (local_1c + -1) * 0x14));
        }
      }
    }
  }
  for (local_14 = 0; local_14 < *(int *)((int)this + 0x16c4); local_14 = local_14 + 1) {
    if (((*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 0xd8) != 0) &&
        (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 0xdc) != 0)) &&
       (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 0x48) == 0)) {
      FUN_00425f03(param_1,*(int **)(*(int *)((int)this + 0x16cc) + local_14 * 4),
                   *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 0xc0) + param_2,
                   *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_14 * 4) + 0xc4) + param_3,
                   1);
    }
  }
  if (param_4 == 0) {
    SelectObject(param_1,local_8);
  }
  return 0;
}

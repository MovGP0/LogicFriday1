/* 00424ceb FUN_00424ceb */

undefined4 __thiscall FUN_00424ceb(void *this,int param_1)

{
  int iVar1;
  int iVar2;
  bool bVar3;
  void *pvVar4;
  int local_2c;
  uint local_28;
  uint local_24;
  int local_20;
  int local_18;
  uint local_10;
  uint local_c;
  
  local_20 = 0;
  for (local_10 = *(uint *)((int)this + 0x1654); local_10 < *(uint *)((int)this + 0x1650);
      local_10 = local_10 + 1) {
    if (*(int *)(local_10 * 0xfc + *(int *)((int)this + 0x3a4)) == 0xb) {
      for (local_24 = *(int *)((int)this + 0x1658); (int)local_24 < *(int *)((int)this + 0x1650);
          local_24 = local_24 + 1) {
        if (*(uint *)(*(int *)((int)this + 0x3a4) + 0x1c + local_24 * 0xfc) == local_10) {
          *(undefined4 *)(*(int *)((int)this + 0x3a4) + 0x1c + local_24 * 0xfc) = 0xfffffffe;
          *(undefined4 *)(*(int *)((int)this + 0x3a4) + 0xbc + local_24 * 0xfc) = 1;
        }
      }
      *(undefined4 *)(*(int *)((int)this + 0x3a4) + 0x48 + local_10 * 0xfc) = 1;
    }
    else if (*(int *)(local_10 * 0xfc + *(int *)((int)this + 0x3a4)) == 10) {
      for (local_24 = *(int *)((int)this + 0x1658); (int)local_24 < *(int *)((int)this + 0x1650);
          local_24 = local_24 + 1) {
        if (*(uint *)(*(int *)((int)this + 0x3a4) + 0x1c + local_24 * 0xfc) == local_10) {
          *(undefined4 *)(*(int *)((int)this + 0x3a4) + 0x1c + local_24 * 0xfc) = 0xffffffff;
          *(undefined4 *)(*(int *)((int)this + 0x3a4) + 0xbc + local_24 * 0xfc) = 1;
        }
      }
      *(undefined4 *)(*(int *)((int)this + 0x3a4) + 0x48 + local_10 * 0xfc) = 1;
    }
    if (((*(int *)(*(int *)((int)this + 0x3a4) + 0x3c + local_10 * 0xfc) == param_1) &&
        (*(int *)(*(int *)((int)this + 0x3a4) + 0xb4 + local_10 * 0xfc) == 0)) &&
       (*(int *)(*(int *)((int)this + 0x3a4) + 0x48 + local_10 * 0xfc) == 0)) {
      bVar3 = false;
      if (*(int *)(*(int *)((int)this + 0x3a4) + 0xf4 + local_10 * 0xfc) != -3) {
        FUN_004258e1(this,local_10,*(int *)(*(int *)((int)this + 0x3a4) + 0xf4 + local_10 * 0xfc),
                     *(int *)(*(int *)((int)this + 0x3a4) + 0xf8 + local_10 * 0xfc));
        *(undefined4 *)(*(int *)((int)this + 0x3a4) + 0xf8 + local_10 * 0xfc) = 0xfffffffd;
        *(undefined4 *)(*(int *)((int)this + 0x3a4) + 0xf4 + local_10 * 0xfc) = 0xfffffffd;
      }
      for (local_28 = 0; local_28 < *(uint *)(*(int *)((int)this + 0x3a4) + 0x18 + local_10 * 0xfc);
          local_28 = local_28 + 1) {
        if ((*(int *)(local_10 * 0xfc + *(int *)((int)this + 0x3a4) + 0x1c + local_28 * 4) != -2) &&
           (*(int *)(local_10 * 0xfc + *(int *)((int)this + 0x3a4) + 0x1c + local_28 * 4) != -1)) {
          local_18 = *(int *)(local_10 * 0xfc + *(int *)((int)this + 0x3a4) + 0x1c + local_28 * 4);
          iVar1 = *(int *)(*(int *)((int)this + 0x3a4) + 0x40 + local_18 * 0xfc);
          if (*(int *)(param_1 * 0x48 + *(int *)(*(int *)((int)this + 0x2678) + iVar1 * 4)) == -1) {
            iVar2 = *(int *)(*(int *)((int)this + 0x3a4) + 0x3c + local_18 * 0xfc);
            bVar3 = true;
            local_24 = param_1;
            do {
              local_24 = local_24 + -1;
              if ((int)local_24 <= iVar2) goto LAB_0042507e;
            } while (*(int *)(local_24 * 0x48 + *(int *)(*(int *)((int)this + 0x2678) + iVar1 * 4))
                     == -1);
            bVar3 = false;
LAB_0042507e:
            if (bVar3) {
              *(uint *)(param_1 * 0x48 + *(int *)(*(int *)((int)this + 0x2678) + iVar1 * 4)) =
                   local_10;
              FUN_0042564a(this,iVar1,param_1,iVar2,local_10,local_28);
              *(undefined4 *)(*(int *)((int)this + 0x3a4) + 0xb4 + local_10 * 0xfc) = 1;
              *(int *)(*(int *)((int)this + 0x3a4) + 0x40 + local_10 * 0xfc) = iVar1;
              break;
            }
          }
        }
      }
      if (!bVar3) {
        local_20 = local_20 + 1;
      }
    }
  }
  if (local_20 != 0) {
    for (local_10 = *(uint *)((int)this + 0x1654); local_10 < *(uint *)((int)this + 0x1650);
        local_10 = local_10 + 1) {
      if (((*(int *)(*(int *)((int)this + 0x3a4) + 0x3c + local_10 * 0xfc) == param_1) &&
          (*(int *)(*(int *)((int)this + 0x3a4) + 0xb4 + local_10 * 0xfc) == 0)) &&
         (*(int *)(*(int *)((int)this + 0x3a4) + 0x48 + local_10 * 0xfc) == 0)) {
        local_c = 0;
        while (((int)local_c < *(int *)(*(int *)((int)this + 0x3a4) + 0x18 + local_10 * 0xfc) &&
               (((*(int *)(local_10 * 0xfc + *(int *)((int)this + 0x3a4) + 0x1c + local_c * 4) == -2
                 || (*(int *)(local_10 * 0xfc + *(int *)((int)this + 0x3a4) + 0x1c + local_c * 4) ==
                     -1)) ||
                (local_18 = *(int *)(local_10 * 0xfc + *(int *)((int)this + 0x3a4) + 0x1c +
                                    local_c * 4),
                *(int *)(*(int *)((int)this + 0x3a4) + 0x3c + local_18 * 0xfc) !=
                *(int *)(*(int *)((int)this + 0x3a4) + 0x3c + local_10 * 0xfc) + -1))))) {
          local_c = local_c + 1;
        }
        local_24 = *(uint *)(*(int *)((int)this + 0x3a4) + 0x40 + local_18 * 0xfc);
        bVar3 = false;
        local_c = local_24;
        do {
          local_c = local_c - 1;
          if ((-1 < (int)local_c) &&
             (*(int *)(param_1 * 0x48 + *(int *)(*(int *)((int)this + 0x2678) + local_c * 4)) == -1)
             ) {
            bVar3 = true;
            local_28 = local_c;
            break;
          }
          local_24 = local_24 + 1;
          if (((int)local_24 < *(int *)((int)this + 0x2670)) &&
             (*(int *)(param_1 * 0x48 + *(int *)(*(int *)((int)this + 0x2678) + local_24 * 4)) == -1
             )) {
            bVar3 = true;
            local_28 = local_24;
            break;
          }
        } while ((0 < (int)local_c) || ((int)local_24 < *(int *)((int)this + 0x2670) + -1));
        local_c = param_1;
        if (!bVar3) {
          *(int *)((int)this + 0x2670) = *(int *)((int)this + 0x2670) + 1;
          pvVar4 = _realloc(*(void **)((int)this + 0x2678),*(int *)((int)this + 0x2670) << 2);
          *(void **)((int)this + 0x2678) = pvVar4;
          if (*(int *)((int)this + 0x2678) == 0) {
            return 0;
          }
          pvVar4 = _malloc(*(int *)((int)this + 0x2674) * 0x48);
          *(void **)(*(int *)((int)this + 0x2678) + -4 + *(int *)((int)this + 0x2670) * 4) = pvVar4;
          if (*(int *)(*(int *)((int)this + 0x2678) + -4 + *(int *)((int)this + 0x2670) * 4) == 0) {
            return 0;
          }
          _memset(*(void **)(*(int *)((int)this + 0x2678) + -4 + *(int *)((int)this + 0x2670) * 4),0
                  ,*(int *)((int)this + 0x2674) * 0x48);
          for (local_2c = 0; local_2c < *(int *)((int)this + 0x2674); local_2c = local_2c + 1) {
            *(undefined4 *)
             (local_2c * 0x48 +
             *(int *)(*(int *)((int)this + 0x2678) + -4 + *(int *)((int)this + 0x2670) * 4)) =
                 0xffffffff;
          }
          local_28 = *(int *)((int)this + 0x2670) - 1;
        }
        do {
          local_c = local_c + -1;
          if ((int)local_c < 0) goto LAB_00425444;
        } while (*(int *)(local_c * 0x48 + *(int *)(*(int *)((int)this + 0x2678) + local_28 * 4)) ==
                 -1);
        bVar3 = true;
LAB_00425444:
        if (bVar3) {
          if (*(int *)(*(int *)((int)this + 0x3a4) + 0x18 + local_10 * 0xfc) == 1) {
            if (*(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_28 * 4) + 4 + local_c * 0x48)
                == 0) {
              *(undefined4 *)
               (*(int *)(*(int *)((int)this + 0x2678) + local_28 * 4) + 4 + param_1 * 0x48) = 0xf;
            }
          }
          else if ((*(int *)(*(int *)((int)this + 0x3a4) + 0x18 + local_10 * 0xfc) == 2) ||
                  (*(int *)(*(int *)((int)this + 0x3a4) + 0x18 + local_10 * 0xfc) == 4)) {
            if (*(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_28 * 4) + 4 + local_c * 0x48)
                == 0xf) {
              *(undefined4 *)
               (*(int *)(*(int *)((int)this + 0x2678) + local_28 * 4) + 4 + param_1 * 0x48) = 0xf;
            }
          }
          else if (*(int *)(*(int *)((int)this + 0x3a4) + 0x18 + local_10 * 0xfc) == 3) {
            if (*(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_28 * 4) + 4 + local_c * 0x48)
                == 0) {
              *(undefined4 *)
               (*(int *)(*(int *)((int)this + 0x2678) + local_28 * 4) + 4 + param_1 * 0x48) = 0x19;
            }
            else if (*(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_28 * 4) + 4 +
                             local_c * 0x48) == 0xf) {
              *(undefined4 *)
               (*(int *)(*(int *)((int)this + 0x2678) + local_28 * 4) + 4 + param_1 * 0x48) = 0x28;
            }
            else {
              *(undefined4 *)
               (*(int *)(*(int *)((int)this + 0x2678) + local_28 * 4) + 4 + param_1 * 0x48) = 0;
            }
          }
        }
        else {
          *(undefined4 *)
           (*(int *)(*(int *)((int)this + 0x2678) + local_28 * 4) + 4 + param_1 * 0x48) = 0;
        }
        *(uint *)(param_1 * 0x48 + *(int *)(*(int *)((int)this + 0x2678) + local_28 * 4)) = local_10
        ;
        *(undefined4 *)(*(int *)((int)this + 0x3a4) + 0xb4 + local_10 * 0xfc) = 1;
        *(uint *)(*(int *)((int)this + 0x3a4) + 0x40 + local_10 * 0xfc) = local_28;
        local_20 = local_20 + -1;
        if (local_20 == 0) {
          return 1;
        }
      }
    }
  }
  return 1;
}

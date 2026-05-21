/* 00431666 FUN_00431666 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

uint __thiscall
FUN_00431666(void *this,undefined4 param_1,LONG param_2,LONG param_3,int param_4,int param_5,
            int param_6)

{
  int iVar1;
  uint unaff_retaddr;
  char local_154 [256];
  uint local_54;
  int local_50;
  POINT local_4c [5];
  int local_20;
  int local_1c;
  uint local_14;
  int local_10;
  int local_c;
  int local_8;
  
  local_54 = DAT_00451a00 ^ unaff_retaddr;
  local_c = 64000;
  local_14 = 0;
  local_4c[0].x = param_2;
  local_4c[0].y = param_3;
  for (local_8 = 0; local_8 < *(int *)((int)this + 0x16c8); local_8 = local_8 + 1) {
    if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x40) == 0) {
      if (((*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x28) == 2) &&
          (**(int **)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) ==
           *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 0x14)))
         && (*(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 4) ==
             *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 0x18))
         ) {
        *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x44) = 0;
      }
      else {
        iVar1 = FUN_0043bad3(*(void **)(*(int *)((int)this + 0x16d0) + local_8 * 4),local_4c);
        if ((iVar1 != 0) && (local_c == 64000)) {
          *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x44) = 1;
          for (local_50 = 0;
              local_50 < *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x28) + -1;
              local_50 = local_50 + 1) {
            if ((local_50 < local_20) || (local_1c <= local_50)) {
              if (param_4 == 0) {
                *(undefined4 *)
                 (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 8 +
                 local_50 * 0x14) = 0;
              }
            }
            else {
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 8 +
               local_50 * 0x14) = 1;
            }
          }
          local_c = local_8;
          local_14 = local_14 + 1;
        }
        if ((*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x44) != 0) &&
           (local_c != local_8)) {
          if (param_4 == 0) {
            *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x44) = 0;
            for (local_50 = 0;
                local_50 <
                *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x28) + -1;
                local_50 = local_50 + 1) {
              *(undefined4 *)
               (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 8 +
               local_50 * 0x14) = 0;
            }
          }
          else {
            local_14 = local_14 + 1;
          }
        }
      }
    }
  }
  if ((local_c == 64000) || (param_5 == 0)) {
    if ((local_c != 64000) && (param_6 != 0)) {
      do {
        local_10 = 0;
        for (local_8 = 0; local_8 < *(int *)((int)this + 0x16c8); local_8 = local_8 + 1) {
          if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x40) == 0) {
            if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x44) == 0) {
              if (((**(int **)(*(int *)((int)this + 0x16d0) + local_8 * 4) == 2) &&
                  (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) +
                                    *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) +
                                            0xc) * 4) + 0x44) != 0)) ||
                 ((*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x14) == 2 &&
                  (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) +
                                    *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) +
                                            0x20) * 4) + 0x44) != 0)))) {
                *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x44) = 1;
                local_10 = 1;
              }
            }
            else {
              if ((**(int **)(*(int *)((int)this + 0x16d0) + local_8 * 4) == 2) &&
                 (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) +
                                   *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) +
                                           0xc) * 4) + 0x44) == 0)) {
                *(undefined4 *)
                 (*(int *)(*(int *)((int)this + 0x16d0) +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0xc) * 4)
                 + 0x44) = 1;
                local_10 = 1;
              }
              if ((*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x14) == 2) &&
                 (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) +
                                   *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) +
                                           0x20) * 4) + 0x44) == 0)) {
                *(undefined4 *)
                 (*(int *)(*(int *)((int)this + 0x16d0) +
                          *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x20) * 4)
                 + 0x44) = 1;
                local_10 = 1;
              }
            }
          }
        }
      } while (local_10 != 0);
      for (local_8 = 0; local_8 < *(int *)((int)this + 0x16c8); local_8 = local_8 + 1) {
        if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x44) != 0) {
          for (local_50 = 0;
              local_50 < *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x28) + -1;
              local_50 = local_50 + 1) {
            *(undefined4 *)
             (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x2c) + 8 +
             local_50 * 0x14) = 1;
          }
        }
      }
    }
  }
  else {
    for (local_8 = 0;
        local_8 < *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_c * 4) + 0x28) + -1;
        local_8 = local_8 + 1) {
      *(undefined4 *)
       (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_c * 4) + 0x2c) + 8 + local_8 * 0x14)
           = 1;
    }
  }
  if ((DAT_00452ef4 != 0) && (local_c != 64000)) {
    FUN_0043ed39(local_154,
                 (byte *)
                 "Wire %d: iIsOutput=%d, iTypeA=%d, iTypeB=%d nodeA = %d, nodeB = %d, iNodeCnt = %d"
                );
    FUN_0040bdc3((LPARAM)local_154);
  }
  FUN_00431daa(this,*(HDC *)((int)this + 0x2318),0,0,0,0);
  return local_14 & 0xffff | local_c << 0x10;
}

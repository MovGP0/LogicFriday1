/* 0042a6d9 FUN_0042a6d9 */

undefined4 __thiscall FUN_0042a6d9(void *this,int param_1)

{
  int *piVar1;
  int iVar2;
  HCURSOR hCursor;
  int iVar3;
  int local_48;
  POINT local_44;
  int local_3c;
  int local_38;
  int local_34;
  int local_10;
  undefined4 local_c;
  int local_8;
  
  local_c = 0;
  for (local_8 = 0; local_8 < *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0x18);
      local_8 = local_8 + 1) {
    if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xe4 + local_8 * 4) == -3) {
      iVar2 = *(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4);
      local_44.x = *(int *)(iVar2 + 0x6c + local_8 * 8);
      local_44.y = *(int *)(iVar2 + 0x70 + local_8 * 8);
      for (local_48 = 0; local_48 < *(int *)((int)this + 0x16c8); local_48 = local_48 + 1) {
        if ((*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x40) == 0) &&
           (iVar2 = FUN_0043bad3(*(void **)(*(int *)((int)this + 0x16d0) + local_48 * 4),&local_44),
           iVar2 != 0)) {
          if (local_10 == 0) {
            if ((local_44.x == local_3c) && (local_44.y == local_38)) {
              *(undefined4 *)((int)this + 0x24ec) = 0;
              *(int *)((int)this + 0x24f0) = param_1;
              *(int *)((int)this + 0x24f4) = local_8;
              *(undefined4 *)((int)this + 0x2500) = 2;
              *(int *)((int)this + 0x250c) = local_48;
              FUN_0043ac51((void *)((int)this + 0x24ec),local_44.x,local_44.y);
              FUN_0043ac51((void *)((int)this + 0x24ec),local_44.x,local_44.y);
              *(undefined4 *)((int)this + 0x2524) =
                   *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x38);
              FUN_00429b01();
              FUN_0043aba6((undefined4 *)((int)this + 0x24ec));
              *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xe4 + local_8 * 4) =
                   *(int *)((int)this + 0x16c8) + -1;
              local_c = 1;
              break;
            }
          }
          else if (local_34 == 0) {
            if (local_44.x == local_3c) {
LAB_0042a7c4:
              if (**(int **)(*(int *)((int)this + 0x16d0) + local_48 * 4) == -3) {
                piVar1 = *(int **)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x2c);
                *piVar1 = local_44.x;
                piVar1[1] = local_44.y;
                **(undefined4 **)(*(int *)((int)this + 0x16d0) + local_48 * 4) = 0;
                *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 4) = param_1;
                *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 8) = local_8;
              }
              else {
                iVar3 = (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x28) + -1
                        ) * 0x14;
                iVar2 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x2c);
                *(LONG *)(iVar2 + iVar3) = local_44.x;
                *(LONG *)(iVar2 + 4 + iVar3) = local_44.y;
                *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x14) = 0;
                *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x18) = param_1;
                *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x1c) = local_8;
              }
              *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x3c) = 0;
              *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xe4 + local_8 * 4) =
                   local_48;
              local_c = 1;
              break;
            }
          }
          else if (local_44.y == local_38) goto LAB_0042a7c4;
        }
      }
    }
  }
  if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xe0) == -3) {
    iVar2 = *(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4);
    local_44.x = *(int *)(iVar2 + 0xac);
    local_44.y = *(int *)(iVar2 + 0xb0);
    for (local_48 = 0; local_48 < *(int *)((int)this + 0x16c8); local_48 = local_48 + 1) {
      if ((*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x40) == 0) &&
         (iVar2 = FUN_0043bad3(*(void **)(*(int *)((int)this + 0x16d0) + local_48 * 4),&local_44),
         iVar2 != 0)) {
        if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x38) != -3) {
          *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xc0) =
               *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xc0) + -5;
          hCursor = LoadCursorA((HINSTANCE)0x0,(LPCSTR)0x7f88);
          SetCursor(hCursor);
          return local_c;
        }
        if (local_10 == 0) {
          if ((local_44.x == local_3c) && (local_44.y == local_38)) {
            *(undefined4 *)((int)this + 0x24ec) = 1;
            *(int *)((int)this + 0x24f0) = param_1;
            *(undefined4 *)((int)this + 0x2500) = 2;
            *(int *)((int)this + 0x250c) = local_48;
            FUN_0043ac51((void *)((int)this + 0x24ec),local_44.x,local_44.y);
            FUN_0043ac51((void *)((int)this + 0x24ec),local_44.x,local_44.y);
            *(int *)((int)this + 0x2524) = param_1;
            FUN_00429b01();
            FUN_0043aba6((undefined4 *)((int)this + 0x24ec));
            *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xe0) =
                 *(int *)((int)this + 0x16c8) + -1;
            return 1;
          }
        }
        else if (local_34 == 0) {
          if (local_44.x == local_3c) {
LAB_0042aad7:
            if (**(int **)(*(int *)((int)this + 0x16d0) + local_48 * 4) == -3) {
              piVar1 = *(int **)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x2c);
              *piVar1 = local_44.x;
              piVar1[1] = local_44.y;
              **(undefined4 **)(*(int *)((int)this + 0x16d0) + local_48 * 4) = 1;
              *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 4) = param_1;
            }
            else {
              iVar3 = (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x28) + -1)
                      * 0x14;
              iVar2 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x2c);
              *(LONG *)(iVar2 + iVar3) = local_44.x;
              *(LONG *)(iVar2 + 4 + iVar3) = local_44.y;
              *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x14) = 1;
              *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x18) = param_1;
            }
            *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x3c) = 0;
            *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_48 * 4) + 0x38) = param_1;
            *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xe0) = local_48;
            return 1;
          }
        }
        else if (local_44.y == local_38) goto LAB_0042aad7;
      }
    }
  }
  return local_c;
}

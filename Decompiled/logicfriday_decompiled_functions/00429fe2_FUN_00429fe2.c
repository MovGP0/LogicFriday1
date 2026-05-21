/* 00429fe2 FUN_00429fe2 */

int __thiscall FUN_00429fe2(void *this,int param_1)

{
  POINT *pPVar1;
  int iVar2;
  int iVar3;
  int iVar4;
  int iVar5;
  POINT PVar6;
  bool bVar7;
  bool bVar8;
  BOOL BVar9;
  HCURSOR hCursor;
  int local_34;
  int local_28;
  int local_24;
  int local_14;
  int local_c;
  
  local_24 = 0;
  bVar8 = false;
  for (local_c = 0; local_c < *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0x18);
      local_c = local_c + 1) {
    if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xe4 + local_c * 4) == -3) {
      iVar2 = *(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4);
      iVar3 = *(int *)(iVar2 + 0x6c + local_c * 8);
      iVar2 = *(int *)(iVar2 + 0x70 + local_c * 8);
      for (local_34 = 0; local_34 < *(int *)((int)this + 0x16c4); local_34 = local_34 + 1) {
        if (((local_34 != param_1) &&
            (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_34 * 4) + 0x48) == 0)) &&
           (PVar6.y = iVar2, PVar6.x = iVar3,
           BVar9 = PtInRect((RECT *)(*(int *)(*(int *)((int)this + 0x16cc) + local_34 * 4) + 200),
                            PVar6), BVar9 != 0)) {
          iVar4 = *(int *)(*(int *)((int)this + 0x16cc) + local_34 * 4);
          iVar5 = *(int *)(iVar4 + 0xac);
          iVar4 = *(int *)(iVar4 + 0xb0);
          if (iVar2 == iVar4) {
            if (iVar5 - iVar3 < 0xb) {
              if ((local_24 == 0) || (iVar5 == iVar3)) {
                local_24 = 1;
                local_14 = local_c;
                *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xc0) =
                     (iVar5 - iVar3) +
                     *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xc0);
                for (local_28 = 0;
                    local_28 < *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0x18)
                    ; local_28 = local_28 + 1) {
                  *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0x6c +
                          local_28 * 8) =
                       (iVar5 - iVar3) +
                       *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0x6c +
                               local_28 * 8);
                }
                *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xac) =
                     (iVar5 - iVar3) +
                     *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xac);
                if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_34 * 4) + 0xe0) == -3) {
                  *(undefined4 *)((int)this + 0x2500) = 1;
                  *(int *)((int)this + 0x2504) = local_34;
                }
                else {
                  *(undefined4 *)((int)this + 0x2500) = 2;
                  *(undefined4 *)((int)this + 0x250c) =
                       *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16cc) + local_34 * 4) + 0xe0)
                  ;
                }
                *(int *)((int)this + 0x2524) = local_34;
                FUN_0043ac51((void *)((int)this + 0x24ec),iVar5,iVar4);
                FUN_0043ac51((void *)((int)this + 0x24ec),iVar5,iVar4);
                *(undefined4 *)((int)this + 0x24ec) = 0;
                *(int *)((int)this + 0x24f0) = param_1;
                *(int *)((int)this + 0x24f4) = local_c;
                FUN_00429b01();
                FUN_0043aba6((undefined4 *)((int)this + 0x24ec));
              }
            }
            else {
              *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xc0) =
                   (iVar5 - iVar3) + 5 +
                   *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xc0);
            }
          }
        }
      }
    }
  }
  iVar2 = *(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4);
  pPVar1 = (POINT *)(iVar2 + 0xac);
  iVar3 = pPVar1->x;
  iVar2 = *(int *)(iVar2 + 0xb0);
  PVar6 = *pPVar1;
  bVar7 = false;
  local_34 = 0;
  while ((local_34 < *(int *)((int)this + 0x16c4) &&
         (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xe0) == -3))) {
    if ((local_34 != param_1) &&
       (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_34 * 4) + 0x48) == 0)) {
      BVar9 = PtInRect((RECT *)(*(int *)(*(int *)((int)this + 0x16cc) + local_34 * 4) + 200),PVar6);
      if (BVar9 != 0) {
        local_c = 0;
LAB_0042a3a8:
        if (local_c < *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_34 * 4) + 0x18)) {
          iVar4 = *(int *)(*(int *)((int)this + 0x16cc) + local_34 * 4);
          iVar5 = *(int *)(iVar4 + 0x6c + local_c * 8);
          iVar4 = *(int *)(iVar4 + 0x70 + local_c * 8);
          if (iVar4 != iVar2) goto LAB_0042a3a1;
          bVar7 = true;
          if ((local_24 != 0) && (iVar5 != iVar3)) {
            *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xc4) =
                 *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xc4) + -5;
            iVar4 = *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xe4 +
                            local_14 * 4);
            *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + iVar4 * 4) + 0x2c) + 4) =
                 *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + iVar4 * 4) + 0x2c) + 4) +
                 -5;
            goto LAB_0042a3a1;
          }
          if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_34 * 4) + 0xe4 + local_c * 4)
              != -3) {
            *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xc4) =
                 *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xc4) + -5;
            hCursor = LoadCursorA((HINSTANCE)0x0,(LPCSTR)0x7f88);
            SetCursor(hCursor);
            goto LAB_0042a3a1;
          }
          if (10 < iVar3 - iVar5) {
            *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xc0) =
                 *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xc0) -
                 ((iVar3 - iVar5) + 5);
            goto LAB_0042a3a1;
          }
          if (iVar3 != iVar5) {
            bVar8 = true;
            for (local_28 = 0;
                local_28 < *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0x18);
                local_28 = local_28 + 1) {
              *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0x6c + local_28 * 8) =
                   *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0x6c +
                           local_28 * 8) - (iVar3 - iVar5);
            }
          }
          *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xc0) =
               *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + param_1 * 4) + 0xc0) -
               (iVar3 - iVar5);
          if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_34 * 4) + 0xe4 + local_c * 4)
              == -3) {
            *(undefined4 *)((int)this + 0x2500) = 0;
            *(int *)((int)this + 0x2504) = local_34;
            *(int *)((int)this + 0x2508) = local_c;
          }
          else {
            *(undefined4 *)((int)this + 0x2500) = 2;
            *(undefined4 *)((int)this + 0x250c) =
                 *(undefined4 *)
                  (*(int *)(*(int *)((int)this + 0x16cc) + local_34 * 4) + 0xe4 + local_c * 4);
          }
          *(int *)((int)this + 0x2524) = param_1;
          FUN_0043ac51((void *)((int)this + 0x24ec),iVar5,iVar4);
          FUN_0043ac51((void *)((int)this + 0x24ec),iVar5,iVar4);
          *(undefined4 *)((int)this + 0x24ec) = 1;
          *(int *)((int)this + 0x24f0) = param_1;
          FUN_00429b01();
          FUN_0043aba6((undefined4 *)((int)this + 0x24ec));
          local_24 = 1;
        }
      }
      if (bVar7) break;
    }
    local_34 = local_34 + 1;
  }
  if (bVar8) {
    FUN_00429fe2(this,param_1);
  }
  return local_24;
LAB_0042a3a1:
  local_c = local_c + 1;
  goto LAB_0042a3a8;
}

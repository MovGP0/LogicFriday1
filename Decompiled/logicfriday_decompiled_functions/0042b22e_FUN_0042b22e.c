/* 0042b22e FUN_0042b22e */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

LRESULT __thiscall FUN_0042b22e(void *this,HWND param_1,uint param_2,uint *param_3,uint *param_4)

{
  int *piVar1;
  undefined4 uVar2;
  short sVar3;
  short sVar4;
  SHORT SVar5;
  UINT_PTR UVar6;
  HGDIOBJ pvVar7;
  int iVar8;
  HCURSOR pHVar9;
  INT_PTR IVar10;
  int iVar11;
  uint uVar12;
  LRESULT LVar13;
  int iVar14;
  uint unaff_retaddr;
  int local_13c;
  int local_138;
  int local_118;
  int local_114;
  int local_110;
  int local_10c;
  int local_108;
  int local_104;
  int local_100;
  int local_fc;
  int local_f8;
  int local_e0;
  int local_dc;
  tagRECT local_d0;
  uint local_c0;
  int local_b8;
  uint local_b4;
  uint local_b0;
  uint local_ac;
  uint local_a8;
  char local_a4 [68];
  LONG local_60;
  LONG local_5c;
  uint local_58;
  tagPAINTSTRUCT local_54;
  uint local_14;
  int local_10;
  HGDIOBJ local_c;
  HGDIOBJ local_8;
  
  local_14 = DAT_00451a00 ^ unaff_retaddr;
  local_ac = 0;
  local_58 = 0;
  local_b4 = 0;
  if (param_2 < 0x116) {
    if (param_2 == 0x115) {
      GetClientRect(param_1,&local_d0);
      *(undefined4 *)((int)this + 0x2374) = 0x17;
      GetScrollInfo(param_1,1,(LPSCROLLINFO)((int)this + 0x2370));
      uVar12 = (uint)param_3 & 0xffff;
      if (uVar12 == 0) {
        if (local_d0.top + *(int *)((int)this + 0x2390) <= *(int *)((int)this + 0x25bc)) {
          return 0;
        }
        local_10 = *(int *)((int)this + 0x23e8);
      }
      else if (uVar12 == 1) {
        if (*(int *)((int)this + 0x25c4) <= local_d0.bottom + *(int *)((int)this + 0x2390)) {
          return 0;
        }
        local_10 = -*(int *)((int)this + 0x23e8);
      }
      else if (uVar12 == 2) {
        if (local_d0.top + *(int *)((int)this + 0x2390) <= *(int *)((int)this + 0x25bc)) {
          return 0;
        }
        if ((local_d0.top + *(int *)((int)this + 0x2390)) - *(int *)((int)this + 0x2380) <
            *(int *)((int)this + 0x25bc)) {
          local_10 = (local_d0.top + *(int *)((int)this + 0x2390)) - *(int *)((int)this + 0x25bc);
        }
        else {
          local_10 = *(int *)((int)this + 0x2380);
        }
      }
      else if (uVar12 == 3) {
        if (*(int *)((int)this + 0x25c4) <= local_d0.bottom + *(int *)((int)this + 0x2390)) {
          return 0;
        }
        if (*(int *)((int)this + 0x25c4) <
            local_d0.bottom + *(int *)((int)this + 0x2390) + *(int *)((int)this + 0x2380)) {
          local_10 = *(int *)((int)this + 0x2390) - (*(int *)((int)this + 0x25c4) - local_d0.bottom)
          ;
        }
        else {
          local_10 = -*(int *)((int)this + 0x2380);
        }
      }
      else {
        if (uVar12 != 4) {
          return 0;
        }
        local_10 = *(int *)((int)this + 0x2384) - *(int *)((int)this + 0x2388);
        *(undefined4 *)((int)this + 0x2384) = *(undefined4 *)((int)this + 0x2388);
      }
      FUN_00431daa(this,*(HDC *)((int)this + 0x2318),0,local_10,0,0);
      *(int *)((int)this + 0x2384) = *(int *)((int)this + 0x2384) - local_10;
      *(int *)((int)this + 0x2390) = *(int *)((int)this + 0x2390) - local_10;
      FUN_0043049d(param_1,(int *)((int)this + 0x25b8),(LPCSCROLLINFO)((int)this + 0x2354),
                   (LPCSCROLLINFO)((int)this + 0x2370),*(int *)((int)this + 0x238c),
                   *(int *)((int)this + 0x2390));
      InvalidateRect(param_1,(RECT *)0x0,0);
      UpdateWindow(param_1);
      return 0;
    }
    if (param_2 < 0x101) {
      if (param_2 == 0x100) {
        if (param_3 == (uint *)0xd) {
          KillTimer(param_1,*(UINT_PTR *)((int)this + 0x25ac));
          local_c0 = FUN_00434112(this);
          if (local_c0 == 0) {
            *(undefined4 *)((int)this + 0x16b4) = 1;
            FUN_004297b5((int)this);
            if (*(int *)((int)this + 0x23d0) == 0) {
              FUN_00423a22((int)this);
            }
            else {
              *(undefined4 *)((int)this + 0x23d0) = 0;
              if ((*(int *)((int)this + 0x2668) == 0) &&
                 (iVar8 = FUN_0042e896(this,*(int *)((int)this + 0x23cc)), iVar8 != 0)) {
                FUN_00430039((int)this);
              }
              else {
                *(undefined4 *)((int)this + 0x23c) = 0;
                FUN_00423a22((int)this);
              }
              *(undefined4 *)((int)this + 0x2668) = 0;
            }
            FUN_0040bde7(0x44ad26);
            SendMessageA(*(HWND *)((int)this + 0x16f0),0x111,0x800f,0);
          }
          else if (local_c0 >> 0x10 < 0x3e9) {
            FUN_0040a274(*(HWND *)((int)this + 0x16f0),local_c0);
          }
          else {
            FUN_00435ac0(this,local_c0);
          }
          return 0;
        }
        if (param_3 == (uint *)0x1b) {
          KillTimer(param_1,*(UINT_PTR *)((int)this + 0x25ac));
          FUN_0040bde7(0x44ad26);
          SendMessageA(*(HWND *)((int)this + 0x16f0),0x111,0x158,0);
          return 0;
        }
        if ((uint *)0x24 < param_3) {
          if (param_3 < (uint *)0x29) {
            if (param_3 == (uint *)0x25) {
              local_a8 = FUN_004326bc(this,param_1,-*(int *)((int)this + 0x23ec),0);
            }
            else if (param_3 == (uint *)0x27) {
              local_a8 = FUN_004326bc(this,param_1,*(int *)((int)this + 0x23ec),0);
            }
            else if (param_3 == (uint *)0x26) {
              local_a8 = FUN_004326bc(this,param_1,0,-*(int *)((int)this + 0x23ec));
            }
            else if (param_3 == (uint *)0x28) {
              local_a8 = FUN_004326bc(this,param_1,0,*(int *)((int)this + 0x23ec));
            }
            if (local_a8 != 0) {
              *(undefined4 *)((int)this + 0x2398) = 1;
              InvalidateRect(param_1,(RECT *)0x0,0);
              UpdateWindow(param_1);
            }
            return 0;
          }
          if (param_3 == (uint *)0x2e) {
            iVar8 = FUN_004306d3();
            if (iVar8 != 0) {
              FUN_00431daa(this,*(HDC *)((int)this + 0x2318),0,0,0,0);
              InvalidateRect(param_1,(RECT *)0x0,0);
              UpdateWindow(param_1);
            }
            return 0;
          }
        }
      }
      else {
        if (param_2 == 1) {
          *(undefined4 *)((int)this + 0x239c) = 0;
          *(undefined4 *)((int)this + 0x2398) = 0;
          *(undefined4 *)((int)this + 0x2394) = 0;
          *(undefined4 *)((int)this + 0x25e8) = 0;
          *(undefined4 *)((int)this + 0x23a8) = 0;
          *(undefined4 *)((int)this + 0x23a4) = 0;
          *(undefined4 *)((int)this + 0x23a0) = 0;
          *(undefined4 *)((int)this + 0x2390) = 0;
          *(undefined4 *)((int)this + 0x238c) = 0;
          *(undefined4 *)((int)this + 0x2310) = 0;
          *(undefined4 *)((int)this + 0x23b8) = 0;
          *(undefined4 *)((int)this + 0x23b4) = 0;
          *(undefined4 *)((int)this + 0x23b0) = 0;
          *(undefined4 *)((int)this + 0x23ac) = 0;
          *(undefined4 *)((int)this + 0x23e4) = 0;
          SetRect((LPRECT)((int)this + 0x23d4),0,0,0,0);
          FUN_0042953c(this,param_1);
          *(undefined4 *)((int)this + 0x2370) = 0x1c;
          *(undefined4 *)((int)this + 0x2354) = 0x1c;
          pHVar9 = LoadCursorA(DAT_00452914,(LPCSTR)0x7f03);
          SetCursor(pHVar9);
          UVar6 = SetTimer(param_1,0,100,(TIMERPROC)0x0);
          *(UINT_PTR *)((int)this + 0x25ac) = UVar6;
          *(undefined4 *)((int)this + 0x23e8) = 10;
          *(undefined4 *)((int)this + 0x23ec) = 5;
          SystemParametersInfoA(0x68,0,&local_b0,0);
          if (local_b0 == 0) {
            *(undefined4 *)((int)this + 0x230c) = 0;
          }
          else {
            *(uint *)((int)this + 0x230c) = 0x78 / local_b0;
          }
          return 0;
        }
        if (param_2 == 5) {
          GetClientRect(param_1,&local_d0);
          if (local_d0.right != local_d0.left) {
            if (*(int *)((int)this + 0x16c4) == 0) {
              *(LONG *)((int)this + 0x25b8) = local_d0.left;
              *(LONG *)((int)this + 0x25bc) = local_d0.top;
              *(LONG *)((int)this + 0x25c0) = local_d0.right;
              *(LONG *)((int)this + 0x25c4) = local_d0.bottom;
            }
            FUN_0043049d(param_1,(int *)((int)this + 0x25b8),(LPCSCROLLINFO)((int)this + 0x2354),
                         (LPCSCROLLINFO)((int)this + 0x2370),*(int *)((int)this + 0x238c),
                         *(int *)((int)this + 0x2390));
            InvalidateRect(param_1,(RECT *)0x0,0);
            UpdateWindow(param_1);
            return 0;
          }
        }
        else {
          if (param_2 == 0xf) {
            BeginPaint(param_1,&local_54);
            GetClientRect(param_1,&local_d0);
            BitBlt(*(HDC *)((int)this + 0x231c),local_54.rcPaint.left,local_54.rcPaint.top,
                   local_54.rcPaint.right - local_54.rcPaint.left,
                   local_54.rcPaint.bottom - local_54.rcPaint.top,*(HDC *)((int)this + 0x2318),
                   local_54.rcPaint.left,local_54.rcPaint.top,0xcc0020);
            if (*(int *)((int)this + 0x23a8) == 0) {
              if (((*(int *)((int)this + 0x25e8) != 0) &&
                  (*(int *)((int)this + 0x25d8) != *(int *)((int)this + 0x25e0))) &&
                 (*(int *)((int)this + 0x25dc) != *(int *)((int)this + 0x25e4))) {
                local_8 = SelectObject(*(HDC *)((int)this + 0x231c),*(HGDIOBJ *)((int)this + 0x2324)
                                      );
                pvVar7 = GetStockObject(5);
                local_c = SelectObject(*(HDC *)((int)this + 0x231c),pvVar7);
                Rectangle(*(HDC *)((int)this + 0x231c),*(int *)((int)this + 0x25d8),
                          *(int *)((int)this + 0x25dc),*(int *)((int)this + 0x25e0),
                          *(int *)((int)this + 0x25e4));
                SelectObject(*(HDC *)((int)this + 0x231c),local_8);
                SelectObject(*(HDC *)((int)this + 0x231c),local_c);
              }
            }
            else {
              local_8 = SelectObject(*(HDC *)((int)this + 0x231c),*(HGDIOBJ *)((int)this + 0x2324));
              MoveToEx(*(HDC *)((int)this + 0x231c),**(int **)((int)this + 0x2518),
                       *(int *)(*(int *)((int)this + 0x2518) + 4),(LPPOINT)0x0);
              for (local_a8 = 1; (int)local_a8 < *(int *)((int)this + 0x2514);
                  local_a8 = local_a8 + 1) {
                LineTo(*(HDC *)((int)this + 0x231c),
                       *(int *)(*(int *)((int)this + 0x2518) + local_a8 * 0x14),
                       *(int *)(*(int *)((int)this + 0x2518) + 4 + local_a8 * 0x14));
              }
              LineTo(*(HDC *)((int)this + 0x231c),*(int *)((int)this + 0x23bc),
                     *(int *)((int)this + 0x23c0));
              LineTo(*(HDC *)((int)this + 0x231c),*(int *)((int)this + 0x23ac),
                     *(int *)((int)this + 0x23b0));
              SelectObject(*(HDC *)((int)this + 0x231c),local_8);
            }
            EndPaint(param_1,&local_54);
            SetFocus(param_1);
            return 0;
          }
          if (param_2 == 0x20) {
            if (*(int *)((int)this + 0x2394) != 0) {
              SetCursor((HCURSOR)0x0);
              return 0;
            }
            if ((*(int *)((int)this + 0x23a4) != 0) || (*(int *)((int)this + 0x23a8) != 0)) {
              pHVar9 = LoadCursorA(DAT_00452914,"WIRECURS");
              SetCursor(pHVar9);
              return 0;
            }
          }
        }
      }
    }
    else if (param_2 == 0x101) {
      if (((uint *)0x24 < param_3) && (param_3 < (uint *)0x29)) {
        if (*(int *)((int)this + 0x2398) != 0) {
          *(undefined4 *)((int)this + 0x2398) = 0;
          for (local_a8 = 0; (int)local_a8 < *(int *)((int)this + 0x16c4); local_a8 = local_a8 + 1)
          {
            if ((*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_a8 * 4) + 0xd8) != 0) &&
               (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_a8 * 4) + 0x48) == 0)) {
              FUN_00429fe2(this,local_a8);
              FUN_0042a6d9(this,local_a8);
            }
          }
          FUN_004305a5(this,(int *)((int)this + 0x25b8),200);
          FUN_0043049d(param_1,(int *)((int)this + 0x25b8),(LPCSCROLLINFO)((int)this + 0x2354),
                       (LPCSCROLLINFO)((int)this + 0x2370),*(int *)((int)this + 0x238c),
                       *(int *)((int)this + 0x2390));
          FUN_00431daa(this,*(HDC *)((int)this + 0x2318),0,0,0,0);
          InvalidateRect(param_1,(RECT *)0x0,0);
          UpdateWindow(param_1);
        }
        return 0;
      }
    }
    else if (param_2 == 0x111) {
      uVar12 = (uint)param_3 & 0xffff;
      if (uVar12 < 0x43a) {
        if (0x437 < uVar12) {
LAB_0042e80b:
          *(undefined4 *)((int)this + 0x2394) = 1;
          *(undefined4 *)((int)this + 0x23a8) = 0;
          *(undefined4 *)((int)this + 0x23a4) = 0;
          *(uint *)((int)this + 0x23c4) = (uint)param_3 & 0xffff;
          FUN_0042af77((undefined4 *)((int)this + 0x23f0),*(undefined4 *)((int)this + 0x23c4));
          return 0;
        }
        if (uVar12 < 0x42b) {
          if ((uVar12 == 0x42a) ||
             ((0x3f3 < uVar12 &&
              ((uVar12 < 0x3fb || ((0x3fb < uVar12 && ((uVar12 < 0x403 || (uVar12 == 0x408))))))))))
          goto LAB_0042e80b;
        }
        else {
          if (uVar12 == 0x42b) {
            *(undefined4 *)((int)this + 0x23a8) = 0;
            *(undefined4 *)((int)this + 0x23a4) = 0;
            *(undefined4 *)((int)this + 0x23a0) = 0;
            *(undefined4 *)((int)this + 0x2398) = 0;
            *(undefined4 *)((int)this + 0x2394) = 0;
            InvalidateRect(param_1,(RECT *)0x0,0);
            UpdateWindow(param_1);
            return 0;
          }
          if (uVar12 == 0x42c) {
            *(undefined4 *)((int)this + 0x23a0) = 0;
            *(undefined4 *)((int)this + 0x2398) = 0;
            *(undefined4 *)((int)this + 0x2394) = 0;
            FUN_0043aba6((undefined4 *)((int)this + 0x24ec));
            *(undefined4 *)((int)this + 0x23a4) = 1;
            for (local_a8 = 0; (int)local_a8 < *(int *)((int)this + 0x16c4); local_a8 = local_a8 + 1
                ) {
              *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16cc) + local_a8 * 4) + 0xd8) = 0;
            }
            for (local_a8 = 0; (int)local_a8 < *(int *)((int)this + 0x16c8); local_a8 = local_a8 + 1
                ) {
              if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_a8 * 4) + 0x44) != 0) {
                *(undefined4 *)(*(int *)(*(int *)((int)this + 0x16d0) + local_a8 * 4) + 0x44) = 0;
                for (local_dc = 0;
                    local_dc <
                    *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_a8 * 4) + 0x28);
                    local_dc = local_dc + 1) {
                  *(undefined4 *)
                   (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_a8 * 4) + 0x2c) + 8 +
                   local_dc * 0x14) = 0;
                }
              }
            }
            FUN_00431daa(this,*(HDC *)((int)this + 0x2318),0,0,0,0);
            InvalidateRect(param_1,(RECT *)0x0,0);
            UpdateWindow(param_1);
            return 0;
          }
          if (uVar12 == 0x430) goto LAB_0042e80b;
        }
      }
      else {
        if (uVar12 == 0x800e) {
LAB_0042e3e7:
          *(uint **)((int)this + 0x23e4) = param_4;
          return 0;
        }
        if (uVar12 == 0x8010) {
          *(undefined4 *)((int)this + 0x23d0) = 1;
          *(uint **)((int)this + 0x23cc) = param_4;
          FUN_0042fb5a(this,*(int *)((int)this + 0x23cc));
          iVar8 = *(int *)((int)this + 0x23cc);
          *(undefined4 *)((int)this + 0x25b8) = *(undefined4 *)(iVar8 + 0x169c);
          *(undefined4 *)((int)this + 0x25bc) = *(undefined4 *)(iVar8 + 0x16a0);
          *(undefined4 *)((int)this + 0x25c0) = *(undefined4 *)(iVar8 + 0x16a4);
          *(undefined4 *)((int)this + 0x25c4) = *(undefined4 *)(iVar8 + 0x16a8);
          FUN_004305a5(this,(int *)((int)this + 0x25b8),200);
          FUN_0043049d(param_1,(int *)((int)this + 0x25b8),(LPCSCROLLINFO)((int)this + 0x2354),
                       (LPCSCROLLINFO)((int)this + 0x2370),*(int *)((int)this + 0x238c),
                       *(int *)((int)this + 0x2390));
          FUN_00431daa(this,*(HDC *)((int)this + 0x2318),10,10,0,0);
          for (local_138 = 0; local_138 < *(int *)((int)this + 0x16c8); local_138 = local_138 + 1) {
            for (local_13c = 0;
                local_13c < *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_138 * 4) + 0x30);
                local_13c = local_13c + 1) {
              iVar14 = *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_138 * 4) +
                                        0x34) + 0x10 + local_13c * 0x14) * 0x14;
              iVar8 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) +
                                       *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) +
                                                                 local_138 * 4) + 0x34) + 0xc +
                                               local_13c * 0x14) * 4) + 0x2c);
              uVar2 = *(undefined4 *)(iVar8 + 4 + iVar14);
              iVar11 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_138 * 4) + 0x34);
              *(undefined4 *)(iVar11 + local_13c * 0x14) = *(undefined4 *)(iVar8 + iVar14);
              *(undefined4 *)(iVar11 + 4 + local_13c * 0x14) = uVar2;
            }
          }
          InvalidateRect(param_1,(RECT *)0x0,0);
          UpdateWindow(param_1);
          return 0;
        }
        if (uVar12 == 0x8011) {
          FUN_0042e94c((int)this);
        }
        else {
          if (uVar12 == 0x8017) {
            local_a8 = 0;
            while( true ) {
              if (*(int *)((int)this + 0x16c4) <= (int)local_a8) {
                local_a8 = 0;
                while( true ) {
                  if (*(int *)((int)this + 0x16c8) <= (int)local_a8) {
                    *param_4 = 0;
                    return 0;
                  }
                  if ((*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_a8 * 4) + 0x44) != 0)
                     && (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_a8 * 4) + 0x40) == 0
                        )) break;
                  local_a8 = local_a8 + 1;
                }
                *param_4 = 1;
                return 0;
              }
              if ((*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_a8 * 4) + 0xd8) != 0) &&
                 (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_a8 * 4) + 0x48) == 0))
              break;
              local_a8 = local_a8 + 1;
            }
            *param_4 = 1;
            return 0;
          }
          if (uVar12 == 0x801e) {
            if (*(int *)((int)this + 0x266c) != 0) {
              *(undefined4 *)((int)this + 0x266c) = 0;
              return 0;
            }
            return 1;
          }
          if (uVar12 == 0x801f) {
            local_a8 = 0;
            while (((int)local_a8 < *(int *)((int)this + 0x16c4) &&
                   (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_a8 * 4) + 0x48) != 0))) {
              local_a8 = local_a8 + 1;
            }
            if ((int)local_a8 < *(int *)((int)this + 0x16c4)) {
              *param_4 = 1;
            }
            else {
              *param_4 = 0;
            }
            goto LAB_0042e3e7;
          }
        }
      }
    }
    else {
      if (param_2 == 0x113) {
        if (*(int *)((int)this + 0x23a8) != 0) {
          GetClientRect(param_1,&local_d0);
          if (*(int *)((int)this + 0x23ac) < *(int *)((int)this + 0x23e8)) {
            if (*(int *)((int)this + 0x2348) == 0) {
              *(int *)((int)this + 0x23bc) =
                   *(int *)((int)this + 0x23bc) + *(int *)((int)this + 0x23e8);
            }
            for (local_a8 = 0; (int)local_a8 < *(int *)((int)this + 0x2514); local_a8 = local_a8 + 1
                ) {
              *(int *)(local_a8 * 0x14 + *(int *)((int)this + 0x2518)) =
                   *(int *)(*(int *)((int)this + 0x2518) + local_a8 * 0x14) +
                   *(int *)((int)this + 0x23e8);
            }
            SendMessageA(param_1,0x114,0,0);
          }
          else if (local_d0.right - *(int *)((int)this + 0x23e8) < *(int *)((int)this + 0x23ac)) {
            if (*(int *)((int)this + 0x2348) == 0) {
              *(int *)((int)this + 0x23bc) =
                   *(int *)((int)this + 0x23bc) - *(int *)((int)this + 0x23e8);
            }
            for (local_a8 = 0; (int)local_a8 < *(int *)((int)this + 0x2514); local_a8 = local_a8 + 1
                ) {
              *(int *)(local_a8 * 0x14 + *(int *)((int)this + 0x2518)) =
                   *(int *)(*(int *)((int)this + 0x2518) + local_a8 * 0x14) -
                   *(int *)((int)this + 0x23e8);
            }
            SendMessageA(param_1,0x114,1,0);
          }
          else if (*(int *)((int)this + 0x23b0) < *(int *)((int)this + 0x23e8)) {
            if (*(int *)((int)this + 0x2348) == 1) {
              *(int *)((int)this + 0x23c0) =
                   *(int *)((int)this + 0x23c0) + *(int *)((int)this + 0x23e8);
            }
            for (local_a8 = 0; (int)local_a8 < *(int *)((int)this + 0x2514); local_a8 = local_a8 + 1
                ) {
              *(int *)(*(int *)((int)this + 0x2518) + 4 + local_a8 * 0x14) =
                   *(int *)(*(int *)((int)this + 0x2518) + 4 + local_a8 * 0x14) +
                   *(int *)((int)this + 0x23e8);
            }
            SendMessageA(param_1,0x115,0,0);
          }
          else if (local_d0.bottom - *(int *)((int)this + 0x23e8) < *(int *)((int)this + 0x23b0)) {
            if (*(int *)((int)this + 0x2348) == 1) {
              *(int *)((int)this + 0x23c0) =
                   *(int *)((int)this + 0x23c0) - *(int *)((int)this + 0x23e8);
            }
            for (local_a8 = 0; (int)local_a8 < *(int *)((int)this + 0x2514); local_a8 = local_a8 + 1
                ) {
              *(int *)(*(int *)((int)this + 0x2518) + 4 + local_a8 * 0x14) =
                   *(int *)(*(int *)((int)this + 0x2518) + 4 + local_a8 * 0x14) -
                   *(int *)((int)this + 0x23e8);
            }
            SendMessageA(param_1,0x115,1,0);
          }
        }
        return 0;
      }
      if (param_2 == 0x114) {
        GetClientRect(param_1,&local_d0);
        *(undefined4 *)((int)this + 0x2358) = 0x17;
        GetScrollInfo(param_1,0,(LPSCROLLINFO)((int)this + 0x2354));
        local_10 = 0;
        uVar12 = (uint)param_3 & 0xffff;
        if (uVar12 == 0) {
          if (local_d0.left + *(int *)((int)this + 0x238c) <= *(int *)((int)this + 0x25b8)) {
            return 0;
          }
          local_e0 = *(int *)((int)this + 0x23e8);
        }
        else if (uVar12 == 1) {
          if (*(int *)((int)this + 0x25c0) <= local_d0.right + *(int *)((int)this + 0x238c)) {
            return 0;
          }
          local_e0 = -*(int *)((int)this + 0x23e8);
        }
        else if (uVar12 == 2) {
          if (local_d0.left + *(int *)((int)this + 0x238c) <= *(int *)((int)this + 0x25b8)) {
            return 0;
          }
          if ((local_d0.left + *(int *)((int)this + 0x238c)) - *(int *)((int)this + 0x2364) <
              *(int *)((int)this + 0x25b8)) {
            local_e0 = (local_d0.left + *(int *)((int)this + 0x238c)) - *(int *)((int)this + 0x25b8)
            ;
          }
          else {
            local_e0 = *(int *)((int)this + 0x2364);
          }
        }
        else if (uVar12 == 3) {
          if (*(int *)((int)this + 0x25c0) <= local_d0.right + *(int *)((int)this + 0x238c)) {
            return 0;
          }
          if (*(int *)((int)this + 0x25c0) <
              local_d0.right + *(int *)((int)this + 0x238c) + *(int *)((int)this + 0x2364)) {
            local_e0 = *(int *)((int)this + 0x238c) -
                       (*(int *)((int)this + 0x25c0) - local_d0.right);
          }
          else {
            local_e0 = -*(int *)((int)this + 0x2364);
          }
        }
        else {
          if (uVar12 != 4) {
            return 0;
          }
          local_e0 = *(int *)((int)this + 0x2368) - *(int *)((int)this + 0x236c);
          *(undefined4 *)((int)this + 0x2368) = *(undefined4 *)((int)this + 0x236c);
        }
        FUN_00431daa(this,*(HDC *)((int)this + 0x2318),local_e0,0,0,0);
        *(int *)((int)this + 0x2368) = *(int *)((int)this + 0x2368) - local_e0;
        *(int *)((int)this + 0x238c) = *(int *)((int)this + 0x238c) - local_e0;
        FUN_0043049d(param_1,(int *)((int)this + 0x25b8),(LPCSCROLLINFO)((int)this + 0x2354),
                     (LPCSCROLLINFO)((int)this + 0x2370),*(int *)((int)this + 0x238c),
                     *(int *)((int)this + 0x2390));
        InvalidateRect(param_1,(RECT *)0x0,0);
        UpdateWindow(param_1);
        return 0;
      }
    }
  }
  else if (param_2 < 0x205) {
    if (param_2 == 0x204) {
      SetFocus(param_1);
      if (*(int *)((int)this + 0x2394) == 0) {
        if (*(int *)((int)this + 0x23a8) != 0) {
          *(undefined4 *)((int)this + 0x23a8) = 0;
          *(undefined4 *)((int)this + 0x23a4) = 1;
          *(undefined4 *)((int)this + 0x266c) = 1;
          FUN_0043aba6((undefined4 *)((int)this + 0x24ec));
        }
      }
      else {
        *(undefined4 *)((int)this + 0x2394) = 0;
        *(undefined4 *)((int)this + 0x266c) = 1;
      }
      if (*(int *)((int)this + 0x266c) != 0) {
        InvalidateRect(param_1,(RECT *)0x0,0);
        UpdateWindow(param_1);
      }
      return 0;
    }
    sVar3 = (short)param_4;
    sVar4 = (short)((uint)param_4 >> 0x10);
    if (param_2 == 0x200) {
      *(int *)((int)this + 0x23ac) = (int)sVar3;
      *(int *)((int)this + 0x23b0) = (int)sVar4;
      *(undefined4 *)((int)this + 0x25d0) = *(undefined4 *)((int)this + 0x23ac);
      *(undefined4 *)((int)this + 0x25d4) = *(undefined4 *)((int)this + 0x23b0);
      *(int *)((int)this + 0x23ac) =
           *(int *)((int)this + 0x23ec) *
           ((*(int *)((int)this + 0x23ac) - *(int *)((int)this + 0x23b4)) /
           *(int *)((int)this + 0x23ec)) + *(int *)((int)this + 0x23b4);
      *(int *)((int)this + 0x23b0) =
           *(int *)((int)this + 0x23ec) *
           ((*(int *)((int)this + 0x23b0) - *(int *)((int)this + 0x23b8)) /
           *(int *)((int)this + 0x23ec)) + *(int *)((int)this + 0x23b8);
      FUN_0043ed39(local_a4,(byte *)"%d, %d");
      FUN_0040bde7((LPARAM)local_a4);
      if (*(int *)((int)this + 0x25e8) != 0) {
        if ((*(int *)((int)this + 0x25c8) != *(int *)((int)this + 0x25d0)) &&
           (*(int *)((int)this + 0x25cc) != *(int *)((int)this + 0x25d4))) {
          if (*(int *)((int)this + 0x25c8) < *(int *)((int)this + 0x25d0)) {
            *(undefined4 *)((int)this + 0x25d8) = *(undefined4 *)((int)this + 0x25c8);
            *(undefined4 *)((int)this + 0x25e0) = *(undefined4 *)((int)this + 0x25d0);
          }
          else {
            *(undefined4 *)((int)this + 0x25d8) = *(undefined4 *)((int)this + 0x25d0);
            *(undefined4 *)((int)this + 0x25e0) = *(undefined4 *)((int)this + 0x25c8);
          }
          if (*(int *)((int)this + 0x25cc) < *(int *)((int)this + 0x25d4)) {
            *(undefined4 *)((int)this + 0x25dc) = *(undefined4 *)((int)this + 0x25cc);
            *(undefined4 *)((int)this + 0x25e4) = *(undefined4 *)((int)this + 0x25d4);
          }
          else {
            *(undefined4 *)((int)this + 0x25dc) = *(undefined4 *)((int)this + 0x25d4);
            *(undefined4 *)((int)this + 0x25e4) = *(undefined4 *)((int)this + 0x25cc);
          }
          InvalidateRect(param_1,(RECT *)0x0,0);
          UpdateWindow(param_1);
        }
        return 0;
      }
      if ((*(int *)((int)this + 0x23ac) == *(int *)((int)this + 0x23b4)) &&
         (*(int *)((int)this + 0x23b0) == *(int *)((int)this + 0x23b8))) {
        return 0;
      }
      iVar8 = *(int *)((int)this + 0x23ac) - *(int *)((int)this + 0x23b4);
      local_b8 = *(int *)((int)this + 0x23b0) - *(int *)((int)this + 0x23b8);
      *(undefined4 *)((int)this + 0x23b4) = *(undefined4 *)((int)this + 0x23ac);
      *(undefined4 *)((int)this + 0x23b8) = *(undefined4 *)((int)this + 0x23b0);
      if (*(int *)((int)this + 0x23a0) == 0) {
        if (*(int *)((int)this + 0x2394) == 0) {
          if (*(int *)((int)this + 0x23a8) != 0) {
            if (*(int *)((int)this + 0x2514) == 1) {
              if (*(int *)((int)this + 0x239c) != 0) {
                *(undefined4 *)((int)this + 0x239c) = 0;
                iVar8 = FUN_0043f3b8(iVar8);
                iVar11 = FUN_0043f3b8(local_b8);
                if (iVar11 < iVar8) {
                  *(undefined4 *)((int)this + 0x2550) = 1;
                  *(undefined4 *)((int)this + 0x2348) = 1;
                }
                else {
                  *(undefined4 *)((int)this + 0x2550) = 0;
                  *(undefined4 *)((int)this + 0x2348) = 0;
                }
              }
              if (*(int *)((int)this + 0x2348) == 1) {
                *(undefined4 *)((int)this + 0x23bc) = *(undefined4 *)((int)this + 0x23ac);
                *(undefined4 *)((int)this + 0x23c0) =
                     *(undefined4 *)(*(int *)((int)this + 0x2518) + 4);
              }
              else {
                *(undefined4 *)((int)this + 0x23bc) = **(undefined4 **)((int)this + 0x2518);
                *(undefined4 *)((int)this + 0x23c0) = *(undefined4 *)((int)this + 0x23b0);
              }
            }
            else {
              iVar8 = *(int *)((int)this + 0x2514) + -1;
              if (*(int *)((*(int *)((int)this + 0x2514) + -2) * 0x14 + *(int *)((int)this + 0x2518)
                          ) == *(int *)(iVar8 * 0x14 + *(int *)((int)this + 0x2518))) {
                *(undefined4 *)((int)this + 0x2348) = 0;
                *(undefined4 *)((int)this + 0x23bc) =
                     *(undefined4 *)(iVar8 * 0x14 + *(int *)((int)this + 0x2518));
                *(undefined4 *)((int)this + 0x23c0) = *(undefined4 *)((int)this + 0x23b0);
              }
              else {
                *(undefined4 *)((int)this + 0x2348) = 1;
                *(undefined4 *)((int)this + 0x23bc) = *(undefined4 *)((int)this + 0x23ac);
                *(undefined4 *)((int)this + 0x23c0) =
                     *(undefined4 *)(*(int *)((int)this + 0x2518) + 4 + iVar8 * 0x14);
              }
            }
            InvalidateRect(param_1,(RECT *)0x0,0);
            UpdateWindow(param_1);
          }
        }
        else {
          InvalidateRect(param_1,(RECT *)((int)this + 0x24b8),0);
          UpdateWindow(param_1);
          pvVar7 = GetStockObject(7);
          local_8 = SelectObject(*(HDC *)((int)this + 0x231c),pvVar7);
          SetTextColor(*(HDC *)((int)this + 0x231c),0);
          FUN_00425f03(*(HDC *)((int)this + 0x231c),(int *)((int)this + 0x23f0),
                       *(int *)((int)this + 0x23ac),*(int *)((int)this + 0x23b0),0);
          SelectObject(*(HDC *)((int)this + 0x231c),local_8);
          SetTextColor(*(HDC *)((int)this + 0x231c),0xdf0000);
        }
      }
      else {
        FUN_004326bc(this,param_1,iVar8,local_b8);
        *(undefined4 *)((int)this + 0x2398) = 1;
        InvalidateRect(param_1,(RECT *)0x0,0);
        UpdateWindow(param_1);
      }
      return 0;
    }
    if (param_2 == 0x201) {
      SetFocus(param_1);
      if (*(int *)((int)this + 0x2394) == 0) {
        if ((*(int *)((int)this + 0x23a4) == 0) && (*(int *)((int)this + 0x23a8) == 0)) {
          local_60 = (LONG)sVar3;
          local_5c = (LONG)sVar4;
          *(LONG *)((int)this + 0x23b4) = local_60;
          *(LONG *)((int)this + 0x23b8) = local_5c;
          iVar8 = *(int *)((int)this + 0x23b4) % *(int *)((int)this + 0x23ec);
          if (*(int *)((int)this + 0x23ec) / 2 < iVar8) {
            *(int *)((int)this + 0x23b4) =
                 (*(int *)((int)this + 0x23ec) - iVar8) + *(int *)((int)this + 0x23b4);
          }
          else {
            *(int *)((int)this + 0x23b4) = *(int *)((int)this + 0x23b4) - iVar8;
          }
          iVar8 = *(int *)((int)this + 0x23b8) % *(int *)((int)this + 0x23ec);
          if (*(int *)((int)this + 0x23ec) / 2 < iVar8) {
            *(int *)((int)this + 0x23b8) =
                 (*(int *)((int)this + 0x23ec) - iVar8) + *(int *)((int)this + 0x23b8);
          }
          else {
            *(int *)((int)this + 0x23b8) = *(int *)((int)this + 0x23b8) - iVar8;
          }
          local_ac = (uint)param_3 & 4;
          local_58 = (uint)param_3 & 8;
          SVar5 = GetKeyState(0x12);
          local_b4 = (int)SVar5 & 0x8000;
          uVar12 = FUN_0043147c(this,param_1,local_60,local_5c,&local_d0.left,local_ac);
          if (uVar12 >> 0x10 != 64000) {
            *(undefined4 *)((int)this + 0x23a0) = 1;
            SetCapture(param_1);
          }
          uVar12 = FUN_00431666(this,param_1,local_60,local_5c,local_ac,local_58,local_b4);
          if ((uVar12 >> 0x10 != 64000) && (*(int *)((int)this + 0x23a0) == 0)) {
            *(undefined4 *)((int)this + 0x23a0) = 1;
            SetCapture(param_1);
          }
          InvalidateRect(param_1,(RECT *)0x0,0);
          UpdateWindow(param_1);
          if (*(int *)((int)this + 0x23a0) == 0) {
            *(int *)((int)this + 0x25c8) = (int)sVar3;
            *(int *)((int)this + 0x25cc) = (int)sVar4;
            *(undefined4 *)((int)this + 0x25d0) = *(undefined4 *)((int)this + 0x25c8);
            *(undefined4 *)((int)this + 0x25d4) = *(undefined4 *)((int)this + 0x25cc);
            *(undefined4 *)((int)this + 0x25e0) = *(undefined4 *)((int)this + 0x25c8);
            *(undefined4 *)((int)this + 0x25d8) = *(undefined4 *)((int)this + 0x25e0);
            *(undefined4 *)((int)this + 0x25e4) = *(undefined4 *)((int)this + 0x25cc);
            *(undefined4 *)((int)this + 0x25dc) = *(undefined4 *)((int)this + 0x25e4);
            *(undefined4 *)((int)this + 0x25e8) = 1;
            SetCapture(param_1);
          }
        }
      }
      else {
        *(undefined4 *)((int)this + 0x2394) = 0;
        if (((*(int *)((int)this + 0x23c4) == 0x438) || (*(int *)((int)this + 0x23c4) == 0x439)) &&
           (local_a8 = DialogBoxParamA(DAT_00452914,"VARNAMEDLG",param_1,FUN_0040adb4,
                                       *(LPARAM *)((int)this + 0x23c4)), local_a8 == 0)) {
          InvalidateRect(param_1,(RECT *)0x0,0);
          UpdateWindow(param_1);
          return 0;
        }
        FUN_0042999a();
        FUN_00429fe2(this,*(int *)((int)this + 0x16c4) + -1);
        FUN_0042a6d9(this,*(int *)((int)this + 0x16c4) + -1);
        if (*(int *)((int)this + 0x23c4) == 0x438) {
          FUN_0043ebd0((uint *)(*(int *)(*(int *)((int)this + 0x16cc) + -4 +
                                        *(int *)((int)this + 0x16c4) * 4) + 0x50),
                       (uint *)((int)this + (*(int *)((int)this + 0xc4) + -1) * 9 + 0x160));
          FUN_0043ebd0((uint *)(*(int *)(*(int *)((int)this + 0x16cc) + -4 +
                                        *(int *)((int)this + 0x16c4) * 4) + 4),
                       (uint *)((int)this + (*(int *)((int)this + 0xc4) + -1) * 9 + 0x160));
        }
        else if (*(int *)((int)this + 0x23c4) == 0x439) {
          FUN_0043ebd0((uint *)(*(int *)(*(int *)((int)this + 0x16cc) + -4 +
                                        *(int *)((int)this + 0x16c4) * 4) + 0x50),
                       (uint *)((int)this + (*(int *)((int)this + 200) + -1) * 9 + 0xd0));
          FUN_0043ebd0((uint *)(*(int *)(*(int *)((int)this + 0x16cc) + -4 +
                                        *(int *)((int)this + 0x16c4) * 4) + 4),
                       (uint *)((int)this + (*(int *)((int)this + 200) + -1) * 9 + 0xd0));
        }
        if (*(int *)((int)this + 0x25c0) <
            *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + -4 + *(int *)((int)this + 0x16c4) * 4)
                    + 0xd0) + 200) {
          *(int *)((int)this + 0x25c0) =
               *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + -4 +
                                *(int *)((int)this + 0x16c4) * 4) + 0xd0) + 200;
        }
        if (*(int *)((int)this + 0x25c4) <
            *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + -4 + *(int *)((int)this + 0x16c4) * 4)
                    + 0xd4) + 200) {
          *(int *)((int)this + 0x25c4) =
               *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + -4 +
                                *(int *)((int)this + 0x16c4) * 4) + 0xd4) + 200;
        }
        if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + -4 + *(int *)((int)this + 0x16c4) * 4)
                    + 200) + -200 < *(int *)((int)this + 0x25b8)) {
          *(int *)((int)this + 0x25b8) =
               *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + -4 +
                                *(int *)((int)this + 0x16c4) * 4) + 200) + -200;
        }
        if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + -4 + *(int *)((int)this + 0x16c4) * 4)
                    + 0xcc) + -200 < *(int *)((int)this + 0x25bc)) {
          *(int *)((int)this + 0x25bc) =
               *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + -4 +
                                *(int *)((int)this + 0x16c4) * 4) + 0xcc) + -200;
        }
        FUN_0043049d(param_1,(int *)((int)this + 0x25b8),(LPCSCROLLINFO)((int)this + 0x2354),
                     (LPCSCROLLINFO)((int)this + 0x2370),*(int *)((int)this + 0x238c),
                     *(int *)((int)this + 0x2390));
        FUN_00431daa(this,*(HDC *)((int)this + 0x2318),0,0,0,0);
        InvalidateRect(param_1,(RECT *)0x0,0);
        UpdateWindow(param_1);
      }
      return 0;
    }
    if (param_2 == 0x202) {
      *(undefined4 *)((int)this + 0x23a0) = 0;
      local_60 = (LONG)sVar3;
      local_5c = (LONG)sVar4;
      if (((*(int *)((int)this + 0x25e8) != 0) &&
          (*(undefined4 *)((int)this + 0x25e8) = 0,
          *(int *)((int)this + 0x25d8) != *(int *)((int)this + 0x25e0))) &&
         (*(int *)((int)this + 0x25dc) != *(int *)((int)this + 0x25e4))) {
        FUN_00431cac((int)this);
        FUN_00431d3b(this,*(int *)((int)this + 0x25d8),*(int *)((int)this + 0x25dc),
                     *(int *)((int)this + 0x25e0),*(int *)((int)this + 0x25e4));
        FUN_00431daa(this,*(HDC *)((int)this + 0x2318),0,0,0,0);
        InvalidateRect(param_1,(RECT *)0x0,0);
        UpdateWindow(param_1);
      }
      ReleaseCapture();
      if (*(int *)((int)this + 0x2398) == 0) {
        if (*(int *)((int)this + 0x23a4) == 0) {
          if (*(int *)((int)this + 0x23a8) != 0) {
            *(undefined4 *)((int)this + 0x25a8) = 0;
            iVar8 = FUN_0042f2a2(*(int *)((int)this + 0x23ac),*(int *)((int)this + 0x23b0),
                                 (int *)((int)this + 0x23bc),(int *)((int)this + 0x24ec),
                                 (int *)((int)this + 0x2574));
            if (iVar8 == -1) {
              pHVar9 = LoadCursorA((HINSTANCE)0x0,(LPCSTR)0x7f88);
              SetCursor(pHVar9);
            }
            else if (iVar8 == 1) {
              *(undefined4 *)((int)this + 0x23a8) = 0;
              if ((*(int *)((int)this + 0x24ec) == 2) &&
                 (*(int *)((int)this + 0x254c) == *(int *)((int)this + 0x2550))) {
                if (*(int *)((int)this + 0x2514) == 1) {
                  local_fc = *(int *)((int)this + 0x23bc);
                  local_f8 = *(int *)((int)this + 0x23c0);
                }
                else {
                  local_fc = *(int *)(*(int *)((int)this + 0x2518) + 0x14);
                  local_f8 = *(int *)(*(int *)((int)this + 0x2518) + 0x18);
                }
                if (*(int *)((int)this + 0x254c) == 1) {
                  if (*(int *)((int)this + 0x2558) < *(int *)((int)this + 0x2560)) {
                    local_104 = *(int *)((int)this + 0x2558);
                    local_100 = *(int *)((int)this + 0x255c);
                    local_118 = *(int *)((int)this + 0x2560);
                    local_114 = *(int *)((int)this + 0x2564);
                  }
                  else {
                    local_104 = *(int *)((int)this + 0x2560);
                    local_100 = *(int *)((int)this + 0x2564);
                    local_118 = *(int *)((int)this + 0x2558);
                    local_114 = *(int *)((int)this + 0x255c);
                  }
                  local_110 = local_104;
                  local_108 = local_118;
                  local_10c = local_fc;
                }
                else {
                  if (*(int *)((int)this + 0x255c) < *(int *)((int)this + 0x2564)) {
                    local_104 = *(int *)((int)this + 0x2558);
                    local_100 = *(int *)((int)this + 0x255c);
                    local_118 = *(int *)((int)this + 0x2560);
                    local_114 = *(int *)((int)this + 0x2564);
                  }
                  else {
                    local_104 = *(int *)((int)this + 0x2560);
                    local_100 = *(int *)((int)this + 0x2564);
                    local_118 = *(int *)((int)this + 0x2558);
                    local_114 = *(int *)((int)this + 0x255c);
                  }
                  local_110 = local_100;
                  local_108 = local_114;
                  local_10c = local_f8;
                }
                if (local_10c < local_110) {
                  piVar1 = *(int **)((int)this + 0x2518);
                  *piVar1 = local_104;
                  piVar1[1] = local_100;
                }
                else if (local_108 < local_10c) {
                  piVar1 = *(int **)((int)this + 0x2518);
                  *piVar1 = local_118;
                  piVar1[1] = local_114;
                }
                else {
                  piVar1 = *(int **)((int)this + 0x2518);
                  *piVar1 = local_fc;
                  piVar1[1] = local_f8;
                }
              }
              FUN_00429b01();
              if (*(int *)((int)this + 0x2570) == 0) {
                if (*(int *)((int)this + 0x25a8) != 0) {
                  FUN_0043cc09(*(void **)(*(int *)((int)this + 0x16d0) +
                                         *(int *)((int)this + 0x258c) * 4),
                               *(int **)(*(int *)((int)this + 0x16d0) + -4 +
                                        *(int *)((int)this + 0x16c8) * 4),param_1);
                }
              }
              else {
                FUN_0043cc09(*(void **)(*(int *)((int)this + 0x16d0) +
                                       *(int *)((int)this + 0x2554) * 4),
                             *(int **)(*(int *)((int)this + 0x16d0) + -4 +
                                      *(int *)((int)this + 0x16c8) * 4),param_1);
                if (*(int *)((int)this + 0x25a8) != 0) {
                  FUN_0043cc09(*(void **)(*(int *)((int)this + 0x16d0) +
                                         *(int *)((int)this + 0x258c) * 4),
                               *(int **)(*(int *)((int)this + 0x16d0) +
                                        *(int *)((int)this + 0x2554) * 4),param_1);
                }
              }
              if ((*(int *)((int)this + 0x2570) != 0) || (*(int *)((int)this + 0x25a8) != 0)) {
                FUN_00431daa(this,*(HDC *)((int)this + 0x2318),0,0,0,0);
              }
              FUN_0043aba6((undefined4 *)((int)this + 0x24ec));
              FUN_0042aca7(this,*(int *)((int)this + 0x16c8) + -1);
              InvalidateRect(param_1,(RECT *)0x0,0);
              UpdateWindow(param_1);
              *(undefined4 *)((int)this + 0x23a4) = 1;
            }
            else if (iVar8 == 0) {
              FUN_0043b238((void *)((int)this + 0x24ec),*(int *)((int)this + 0x23bc),
                           *(int *)((int)this + 0x23c0),*(int *)((int)this + 0x23ac),
                           *(int *)((int)this + 0x23b0));
            }
          }
        }
        else {
          *(undefined4 *)((int)this + 0x2570) = 0;
          iVar8 = FUN_0042edfd(this,*(LONG *)((int)this + 0x23ac),*(LONG *)((int)this + 0x23b0),
                               (undefined4 *)((int)this + 0x24ec),(LONG *)((int)this + 0x23c8),
                               (POINT *)((int)this + 0x253c));
          if (iVar8 != 0) {
            *(undefined4 *)((int)this + 0x23a4) = 0;
            *(undefined4 *)((int)this + 0x23a8) = 1;
            *(undefined4 *)((int)this + 0x239c) = 1;
          }
        }
      }
      else {
        *(undefined4 *)((int)this + 0x2398) = 0;
        for (local_a8 = 0; (int)local_a8 < *(int *)((int)this + 0x16c4); local_a8 = local_a8 + 1) {
          if ((*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_a8 * 4) + 0xd8) != 0) &&
             (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_a8 * 4) + 0x48) == 0)) {
            FUN_00429fe2(this,local_a8);
            FUN_0042a6d9(this,local_a8);
          }
        }
        FUN_004305a5(this,(int *)((int)this + 0x25b8),200);
        FUN_0043049d(param_1,(int *)((int)this + 0x25b8),(LPCSCROLLINFO)((int)this + 0x2354),
                     (LPCSCROLLINFO)((int)this + 0x2370),*(int *)((int)this + 0x238c),
                     *(int *)((int)this + 0x2390));
        FUN_00431daa(this,*(HDC *)((int)this + 0x2318),0,0,0,0);
        InvalidateRect(param_1,(RECT *)0x0,0);
        UpdateWindow(param_1);
      }
      return 0;
    }
    if (param_2 == 0x203) {
      local_60 = (LONG)sVar3;
      local_5c = (LONG)sVar4;
      uVar12 = FUN_0043147c(this,param_1,local_60,local_5c,&local_d0.left,0);
      FUN_00431daa(this,*(HDC *)((int)this + 0x2318),0,0,0,0);
      InvalidateRect(param_1,(RECT *)0x0,0);
      UpdateWindow(param_1);
      local_a8 = uVar12 >> 0x10;
      if (((int)local_a8 < *(int *)((int)this + 0x16c4)) &&
         (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_a8 * 4) + 0x48) == 0)) {
        if ((**(int **)(*(int *)((int)this + 0x16cc) + local_a8 * 4) == 8) ||
           (**(int **)(*(int *)((int)this + 0x16cc) + local_a8 * 4) == 9)) {
          IVar10 = DialogBoxParamA(DAT_00452914,"VARNAMEDLG",param_1,FUN_0040adf6,local_a8);
          if (IVar10 != 0) {
            FUN_00431daa(this,*(HDC *)((int)this + 0x2318),0,0,0,0);
            InvalidateRect(param_1,(RECT *)0x0,0);
            UpdateWindow(param_1);
          }
        }
        else {
          IVar10 = DialogBoxParamA(DAT_00452914,"GATELABELDLG",param_1,FUN_0040ae38,local_a8);
          if (IVar10 != 0) {
            FUN_00431daa(this,*(HDC *)((int)this + 0x2318),0,0,0,0);
            InvalidateRect(param_1,(RECT *)0x0,0);
            UpdateWindow(param_1);
          }
        }
      }
      return 0;
    }
  }
  else if (param_2 == 0x20a) {
    if (*(int *)((int)this + 0x230c) != 0) {
      *(int *)((int)this + 0x2310) =
           (int)(short)((uint)param_3 >> 0x10) + *(int *)((int)this + 0x2310);
      while (*(int *)((int)this + 0x230c) <= *(int *)((int)this + 0x2310)) {
        SendMessageA(param_1,0x115,0,0);
        *(int *)((int)this + 0x2310) = *(int *)((int)this + 0x2310) - *(int *)((int)this + 0x230c);
      }
      while (*(int *)((int)this + 0x2310) <= -*(int *)((int)this + 0x230c)) {
        SendMessageA(param_1,0x115,1,0);
        *(int *)((int)this + 0x2310) = *(int *)((int)this + 0x2310) + *(int *)((int)this + 0x230c);
      }
      return 0;
    }
  }
  else {
    if (param_2 == 0x800d) {
      *param_3 = *(uint *)(*(int *)((int)this + 0x16d0) + (int)param_4 * 4);
      return 0;
    }
    if (param_2 == 0x8012) {
      *param_4 = *(uint *)((int)this + 0x16d0);
      *param_3 = *(uint *)((int)this + 0x16c8);
      return 0;
    }
  }
  LVar13 = DefWindowProcA(param_1,param_2,(WPARAM)param_3,(LPARAM)param_4);
  return LVar13;
}

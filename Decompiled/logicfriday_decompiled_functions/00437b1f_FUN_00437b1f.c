/* 00437b1f FUN_00437b1f */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

LRESULT __thiscall FUN_00437b1f(void *this,HWND param_1,uint param_2,uint param_3,int *param_4)

{
  int iVar1;
  HCURSOR pHVar2;
  uint uVar3;
  LRESULT LVar4;
  ulonglong uVar5;
  uint unaff_retaddr;
  uint local_b0;
  int local_ac;
  int local_a4;
  HDC local_a0;
  int local_9c;
  int local_98;
  int local_94;
  int local_90;
  uint local_8c;
  float local_88;
  int local_84;
  int local_80;
  float local_7c;
  int local_78;
  int local_74;
  int local_70;
  tagPAINTSTRUCT local_6c;
  uint local_2c;
  RECT local_28;
  tagRECT local_18;
  float local_8;
  
  local_2c = DAT_00451a00 ^ unaff_retaddr;
  if (param_2 < 0x112) {
    if (param_2 == 0x111) {
      uVar3 = param_3 & 0xffff;
      if (0x152 < uVar3) {
        if (uVar3 < 0x156) {
          GetClientRect(param_1,&local_18);
          local_84 = local_18.right / 2;
          local_80 = local_18.bottom / 2;
          uVar3 = param_3 & 0xffff;
          if (uVar3 == 0x153) {
            uVar5 = FUN_0043ee30();
            local_78 = (int)uVar5;
            if (*(int *)((int)this + 0x26a4) - *(int *)((int)this + 0x269c) < local_78) {
              return 0;
            }
            if (*(int *)((int)this + 0x2694) < local_18.right) {
              local_98 = 0;
            }
            else {
              uVar5 = FUN_0043ee30();
              local_98 = (int)uVar5;
            }
            if (*(int *)((int)this + 0x2698) < local_18.bottom) {
              local_94 = 0;
            }
            else {
              uVar5 = FUN_0043ee30();
              local_94 = (int)uVar5;
            }
            uVar5 = FUN_0043ee30();
            *(int *)((int)this + 0x268c) = (int)uVar5 - local_98;
            uVar5 = FUN_0043ee30();
            *(int *)((int)this + 0x2690) = (int)uVar5 - local_94;
            uVar5 = FUN_0043ee30();
            *(int *)((int)this + 0x2694) = (int)uVar5 - local_98;
            uVar5 = FUN_0043ee30();
            *(int *)((int)this + 0x2698) = (int)uVar5 - local_94;
            *(undefined4 *)((int)this + 0x16ac) = 1;
          }
          else if (uVar3 == 0x154) {
            if ((((-1 < *(int *)((int)this + 0x268c)) &&
                 (*(int *)((int)this + 0x2694) <= local_18.right)) &&
                (-1 < *(int *)((int)this + 0x2690))) &&
               (*(int *)((int)this + 0x2698) <= local_18.bottom)) {
              return 0;
            }
            uVar5 = FUN_0043ee30();
            local_98 = (int)uVar5;
            uVar5 = FUN_0043ee30();
            local_94 = (int)uVar5;
            uVar5 = FUN_0043ee30();
            *(int *)((int)this + 0x268c) = (int)uVar5 - local_98;
            uVar5 = FUN_0043ee30();
            *(int *)((int)this + 0x2694) = (int)uVar5 - local_98;
            local_78 = *(int *)((int)this + 0x2694) - *(int *)((int)this + 0x268c);
            if (*(int *)((int)this + 0x268c) < 1) {
              if (*(int *)((int)this + 0x2694) < local_18.right) {
                *(LONG *)((int)this + 0x2694) = local_18.right;
                *(int *)((int)this + 0x268c) = *(int *)((int)this + 0x2694) - local_78;
              }
            }
            else {
              *(undefined4 *)((int)this + 0x268c) = 0;
              *(int *)((int)this + 0x2694) = local_78;
            }
            uVar5 = FUN_0043ee30();
            *(int *)((int)this + 0x2690) = (int)uVar5 - local_94;
            uVar5 = FUN_0043ee30();
            *(int *)((int)this + 0x2698) = (int)uVar5 - local_94;
            local_90 = *(int *)((int)this + 0x2698) - *(int *)((int)this + 0x2690);
            if (*(int *)((int)this + 0x2690) < 1) {
              if (*(int *)((int)this + 0x2698) < local_18.bottom) {
                *(LONG *)((int)this + 0x2698) = local_18.bottom;
                *(int *)((int)this + 0x2690) = *(int *)((int)this + 0x2698) - local_90;
              }
            }
            else {
              *(undefined4 *)((int)this + 0x2690) = 0;
              *(int *)((int)this + 0x2698) = local_90;
            }
            if (((-1 < *(int *)((int)this + 0x268c)) &&
                (*(int *)((int)this + 0x2694) <= local_18.right)) &&
               ((-1 < *(int *)((int)this + 0x2690) &&
                (*(int *)((int)this + 0x2698) <= local_18.bottom)))) {
              PostMessageA(param_1,0x111,0x155,0);
              return 0;
            }
            *(undefined4 *)((int)this + 0x16ac) = 1;
          }
          else if (uVar3 == 0x155) {
            local_8 = (float)((local_18.right + -10) - *(int *)((int)this + 0x2684)) /
                      (float)*(int *)((int)this + 0x26a4);
            local_7c = (float)((local_18.bottom + -10) - *(int *)((int)this + 0x2688)) /
                       (float)*(int *)((int)this + 0x26a8);
            local_88 = local_7c;
            if (local_8 < local_7c) {
              local_88 = local_8;
            }
            *(undefined4 *)((int)this + 0x2690) = 0;
            *(undefined4 *)((int)this + 0x268c) = 0;
            uVar5 = FUN_0043ee30();
            *(int *)((int)this + 0x2694) = (int)uVar5;
            uVar5 = FUN_0043ee30();
            *(int *)((int)this + 0x2698) = (int)uVar5;
            *(undefined4 *)((int)this + 0x16ac) = 0;
          }
          InvalidateRect(param_1,(RECT *)0x0,1);
          UpdateWindow(param_1);
          *(undefined4 *)((int)this + 0x168c) = *(undefined4 *)((int)this + 0x268c);
          *(undefined4 *)((int)this + 0x1690) = *(undefined4 *)((int)this + 0x2690);
          *(undefined4 *)((int)this + 0x1694) = *(undefined4 *)((int)this + 0x2694);
          *(undefined4 *)((int)this + 0x1698) = *(undefined4 *)((int)this + 0x2698);
          PostMessageA(param_1,5,0,0);
          FUN_0040cf1c();
          return 0;
        }
        if (uVar3 == 0x800b) {
          *(HWND *)((int)this + 0x16f4) = param_1;
          *(undefined4 *)((int)this + 0x2370) = 0x1c;
          *(undefined4 *)((int)this + 0x2354) = 0x1c;
          iVar1 = GetSystemMetrics(2);
          *(int *)((int)this + 0x2684) = iVar1;
          iVar1 = GetSystemMetrics(3);
          *(int *)((int)this + 0x2688) = iVar1;
          *(undefined4 *)((int)this + 0x268c) = *(undefined4 *)((int)this + 0x169c);
          *(undefined4 *)((int)this + 0x2690) = *(undefined4 *)((int)this + 0x16a0);
          *(undefined4 *)((int)this + 0x2694) = *(undefined4 *)((int)this + 0x16a4);
          *(undefined4 *)((int)this + 0x2698) = *(undefined4 *)((int)this + 0x16a8);
          *(undefined4 *)((int)this + 0x269c) = *(undefined4 *)((int)this + 0x268c);
          *(undefined4 *)((int)this + 0x26a0) = *(undefined4 *)((int)this + 0x2690);
          *(undefined4 *)((int)this + 0x26a4) = *(undefined4 *)((int)this + 0x2694);
          *(undefined4 *)((int)this + 0x26a8) = *(undefined4 *)((int)this + 0x2698);
          FUN_00424322(this,&local_a4);
          *(undefined4 *)((int)this + 0x2310) = 0;
          if (local_a4 == 0) {
            *(undefined4 *)((int)this + 0x268c) = *(undefined4 *)((int)this + 0x168c);
            *(undefined4 *)((int)this + 0x2690) = *(undefined4 *)((int)this + 0x1690);
            *(undefined4 *)((int)this + 0x2694) = *(undefined4 *)((int)this + 0x1694);
            *(undefined4 *)((int)this + 0x2698) = *(undefined4 *)((int)this + 0x1698);
            InvalidateRect(param_1,(RECT *)0x0,1);
            UpdateWindow(param_1);
            PostMessageA(param_1,5,0,0);
          }
          else {
            PostMessageA(param_1,0x111,0x155,0);
          }
          SystemParametersInfoA(0x68,0,&local_8c,0);
          if (local_8c == 0) {
            *(undefined4 *)((int)this + 0x230c) = 0;
          }
          else {
            *(uint *)((int)this + 0x230c) = 0x78 / local_8c;
          }
          return 0;
        }
        if (uVar3 == 0x8013) {
          *(undefined4 *)((int)this + 0x16b8) = 1;
          *(undefined4 *)((int)this + 0x16bc) = 0;
          return 0;
        }
        if (uVar3 == 0x8014) {
          *(undefined4 *)((int)this + 0x16b8) = 0;
          *(undefined4 *)((int)this + 0x16bc) = 0;
          for (local_74 = 0; local_74 < *(int *)((int)this + 0x1650); local_74 = local_74 + 1) {
            FUN_0041770d((int *)(local_74 * 0xfc + *(int *)((int)this + 0x3a4)));
          }
          FUN_004373f1(this,0);
          InvalidateRect(param_1,(RECT *)0x0,1);
          UpdateWindow(param_1);
          return 0;
        }
        if (uVar3 == 0x8016) {
          local_ac = 0;
          GetClientRect(param_1,&local_18);
          uVar5 = FUN_0043ee30();
          local_b0 = (uint)((int)uVar5 <=
                           *(int *)((int)this + 0x26a4) - *(int *)((int)this + 0x269c));
          if (((*(int *)((int)this + 0x268c) < 0) || (local_18.right < *(int *)((int)this + 0x2694))
              ) || ((*(int *)((int)this + 0x2690) < 0 ||
                    (local_18.bottom < *(int *)((int)this + 0x2698))))) {
            local_ac = 1;
          }
          *param_4 = local_ac * 0x10000 + local_b0;
          return 0;
        }
      }
    }
    else {
      if (param_2 == 1) {
        LVar4 = DefWindowProcA(param_1,1,param_3,(LPARAM)param_4);
        return LVar4;
      }
      if (param_2 == 5) {
        if ((*(int *)((int)this + 0x16ac) == 0) && (param_3 == 2)) {
          PostMessageA(param_1,0x111,0x155,0);
        }
        GetClientRect(param_1,&local_18);
        if ((local_18.right < *(int *)((int)this + 0x2694)) ||
           (*(int *)((int)this + 0x268c) < local_18.left)) {
          *(undefined4 *)((int)this + 0x2358) = 7;
          *(undefined4 *)((int)this + 0x235c) = 0;
          *(int *)((int)this + 0x2360) = *(int *)((int)this + 0x2694) - *(int *)((int)this + 0x268c)
          ;
          iVar1 = FUN_0043f3b8(*(int *)((int)this + 0x268c));
          *(int *)((int)this + 0x2368) = iVar1;
          *(LONG *)((int)this + 0x2364) = local_18.right - *(int *)((int)this + 0x2684);
          if (*(int *)((int)this + 0x2360) - *(int *)((int)this + 0x2364) <
              *(int *)((int)this + 0x2368)) {
            *(int *)((int)this + 0x2364) =
                 *(int *)((int)this + 0x2360) - *(int *)((int)this + 0x2368);
          }
          SetScrollInfo(param_1,0,(LPCSCROLLINFO)((int)this + 0x2354),1);
          ShowScrollBar(param_1,0,1);
        }
        else {
          ShowScrollBar(param_1,0,0);
        }
        if ((local_18.bottom < *(int *)((int)this + 0x2698)) ||
           (*(int *)((int)this + 0x2690) < local_18.top)) {
          *(undefined4 *)((int)this + 0x2374) = 7;
          *(undefined4 *)((int)this + 0x2378) = 0;
          *(int *)((int)this + 0x237c) = *(int *)((int)this + 0x2698) - *(int *)((int)this + 0x2690)
          ;
          iVar1 = FUN_0043f3b8(*(int *)((int)this + 0x2690));
          *(int *)((int)this + 0x2384) = iVar1;
          *(LONG *)((int)this + 0x2380) = local_18.bottom - *(int *)((int)this + 0x2688);
          if (*(int *)((int)this + 0x237c) - *(int *)((int)this + 0x2380) <
              *(int *)((int)this + 0x2384)) {
            *(int *)((int)this + 0x2380) =
                 *(int *)((int)this + 0x237c) - *(int *)((int)this + 0x2384);
          }
          SetScrollInfo(param_1,1,(LPCSCROLLINFO)((int)this + 0x2370),1);
          ShowScrollBar(param_1,1,1);
        }
        else {
          ShowScrollBar(param_1,1,0);
        }
        return 0;
      }
      if (param_2 == 0xf) {
        local_28.left = *(int *)((int)this + 0x268c);
        local_28.top = *(int *)((int)this + 0x2690);
        local_28.right = *(int *)((int)this + 0x2694);
        local_28.bottom = *(int *)((int)this + 0x2698);
        if (((*(int *)((int)this + 0x268c) == 0) && (*(int *)((int)this + 0x2690) == 0)) &&
           (*(int *)((int)this + 0x26a4) < local_28.right)) {
          local_28.left = *(int *)((int)this + 0x269c);
          local_28.top = *(int *)((int)this + 0x26a0);
          local_28.right = *(int *)((int)this + 0x26a4);
          local_28.bottom = *(int *)((int)this + 0x26a8);
        }
        local_28.bottom = local_28.bottom + 10;
        local_28.left = local_28.left + 10;
        local_28.right = local_28.right + 10;
        local_28.top = local_28.top + 10;
        local_a0 = BeginPaint(param_1,&local_6c);
        PlayEnhMetaFile(local_a0,*(HENHMETAFILE *)((int)this + 0x16b0),&local_28);
        EndPaint(param_1,&local_6c);
        return 0;
      }
      if (param_2 == 0x20) {
        if (((DAT_00452e7c == 0) && (DAT_00452eb0 == 0)) &&
           ((DAT_00452eb4 == 0 && ((DAT_00452ed0 == 0 && (DAT_00452ed4 == 0)))))) {
          pHVar2 = LoadCursorA((HINSTANCE)0x0,(LPCSTR)0x7f00);
          SetCursor(pHVar2);
        }
        else {
          pHVar2 = LoadCursorA((HINSTANCE)0x0,(LPCSTR)0x7f02);
          SetCursor(pHVar2);
        }
        return 0;
      }
      if (param_2 == 0x100) {
        if (param_3 == 0x25) {
          PostMessageA(param_1,0x114,0,0);
        }
        else if (param_3 == 0x26) {
          PostMessageA(param_1,0x115,0,0);
        }
        else if (param_3 == 0x27) {
          PostMessageA(param_1,0x114,1,0);
        }
        else if (param_3 == 0x28) {
          PostMessageA(param_1,0x115,1,0);
        }
        return 0;
      }
      if (param_2 == 0x102) {
        if (param_3 == 3) {
          PostMessageA(*(HWND *)((int)this + 0x16f0),0x111,0x151,0);
        }
        return 0;
      }
    }
  }
  else {
    if (param_2 == 0x114) {
      GetClientRect(param_1,&local_18);
      local_18.right = local_18.right - *(int *)((int)this + 0x2684);
      uVar3 = param_3 & 0xffff;
      if (uVar3 == 0) {
        if (-1 < *(int *)((int)this + 0x268c)) {
          return 0;
        }
        local_9c = (*(int *)((int)this + 0x2694) - *(int *)((int)this + 0x268c)) / 100;
        *(int *)((int)this + 0x268c) = *(int *)((int)this + 0x268c) + local_9c;
        *(int *)((int)this + 0x2694) = *(int *)((int)this + 0x2694) + local_9c;
      }
      else if (uVar3 == 1) {
        if (*(int *)((int)this + 0x2694) <= local_18.right) {
          return 0;
        }
        local_9c = (*(int *)((int)this + 0x2694) - *(int *)((int)this + 0x268c)) / 100;
        *(int *)((int)this + 0x268c) = *(int *)((int)this + 0x268c) - local_9c;
        *(int *)((int)this + 0x2694) = *(int *)((int)this + 0x2694) - local_9c;
      }
      else if (uVar3 == 2) {
        if (-1 < *(int *)((int)this + 0x268c)) {
          return 0;
        }
        local_9c = local_18.right;
        if (0 < *(int *)((int)this + 0x268c) + local_18.right) {
          local_9c = FUN_0043f3b8(*(int *)((int)this + 0x268c));
        }
        *(int *)((int)this + 0x268c) = *(int *)((int)this + 0x268c) + local_9c;
        *(int *)((int)this + 0x2694) = *(int *)((int)this + 0x2694) + local_9c;
      }
      else if (uVar3 == 3) {
        if (*(int *)((int)this + 0x2694) <= local_18.right) {
          return 0;
        }
        local_9c = local_18.right;
        if (*(int *)((int)this + 0x2694) - local_18.right <= local_18.right) {
          local_9c = *(int *)((int)this + 0x2694) - local_18.right;
        }
        *(int *)((int)this + 0x268c) = *(int *)((int)this + 0x268c) - local_9c;
        *(int *)((int)this + 0x2694) = *(int *)((int)this + 0x2694) - local_9c;
      }
      else {
        if (uVar3 != 4) {
          return 0;
        }
        *(undefined4 *)((int)this + 0x2358) = 0x10;
        GetScrollInfo(param_1,0,(LPSCROLLINFO)((int)this + 0x2354));
        local_70 = *(int *)((int)this + 0x236c);
        SetScrollPos(param_1,0,local_70,1);
        local_78 = *(int *)((int)this + 0x2694) - *(int *)((int)this + 0x268c);
        *(int *)((int)this + 0x268c) = -local_70;
        *(int *)((int)this + 0x2694) = local_78 - local_70;
      }
      InvalidateRect(param_1,(RECT *)0x0,1);
      UpdateWindow(param_1);
      PostMessageA(param_1,5,0,0);
      *(undefined4 *)((int)this + 0x168c) = *(undefined4 *)((int)this + 0x268c);
      *(undefined4 *)((int)this + 0x1690) = *(undefined4 *)((int)this + 0x2690);
      *(undefined4 *)((int)this + 0x1694) = *(undefined4 *)((int)this + 0x2694);
      *(undefined4 *)((int)this + 0x1698) = *(undefined4 *)((int)this + 0x2698);
      return 0;
    }
    if (param_2 == 0x115) {
      GetClientRect(param_1,&local_18);
      local_18.bottom = local_18.bottom - *(int *)((int)this + 0x2688);
      uVar3 = param_3 & 0xffff;
      if (uVar3 == 0) {
        if (-1 < *(int *)((int)this + 0x2690)) {
          return 0;
        }
        local_9c = (*(int *)((int)this + 0x2698) - *(int *)((int)this + 0x2690)) / 100;
        *(int *)((int)this + 0x2690) = *(int *)((int)this + 0x2690) + local_9c;
        *(int *)((int)this + 0x2698) = *(int *)((int)this + 0x2698) + local_9c;
      }
      else if (uVar3 == 1) {
        if (*(int *)((int)this + 0x2698) <= local_18.bottom) {
          return 0;
        }
        local_9c = (*(int *)((int)this + 0x2698) - *(int *)((int)this + 0x2690)) / 100;
        *(int *)((int)this + 0x2690) = *(int *)((int)this + 0x2690) - local_9c;
        *(int *)((int)this + 0x2698) = *(int *)((int)this + 0x2698) - local_9c;
      }
      else if (uVar3 == 2) {
        if (-1 < *(int *)((int)this + 0x2690)) {
          return 0;
        }
        local_9c = local_18.bottom;
        if (0 < *(int *)((int)this + 0x2690) + local_18.bottom) {
          local_9c = FUN_0043f3b8(*(int *)((int)this + 0x2690));
        }
        *(int *)((int)this + 0x2690) = *(int *)((int)this + 0x2690) + local_9c;
        *(int *)((int)this + 0x2698) = *(int *)((int)this + 0x2698) + local_9c;
      }
      else if (uVar3 == 3) {
        if (*(int *)((int)this + 0x2698) <= local_18.bottom) {
          return 0;
        }
        local_9c = local_18.bottom;
        if (*(int *)((int)this + 0x2698) - local_18.bottom <= local_18.bottom) {
          local_9c = *(int *)((int)this + 0x2698) - local_18.bottom;
        }
        *(int *)((int)this + 0x2690) = *(int *)((int)this + 0x2690) - local_9c;
        *(int *)((int)this + 0x2698) = *(int *)((int)this + 0x2698) - local_9c;
      }
      else {
        if (uVar3 != 4) {
          return 0;
        }
        *(undefined4 *)((int)this + 0x2374) = 0x10;
        GetScrollInfo(param_1,1,(LPSCROLLINFO)((int)this + 0x2370));
        local_70 = *(int *)((int)this + 0x2388);
        SetScrollPos(param_1,1,local_70,1);
        local_90 = *(int *)((int)this + 0x2698) - *(int *)((int)this + 0x2690);
        *(int *)((int)this + 0x2690) = -local_70;
        *(int *)((int)this + 0x2698) = local_90 - local_70;
      }
      InvalidateRect(param_1,(RECT *)0x0,1);
      UpdateWindow(param_1);
      PostMessageA(param_1,5,0,0);
      *(undefined4 *)((int)this + 0x168c) = *(undefined4 *)((int)this + 0x268c);
      *(undefined4 *)((int)this + 0x1690) = *(undefined4 *)((int)this + 0x2690);
      *(undefined4 *)((int)this + 0x1694) = *(undefined4 *)((int)this + 0x2694);
      *(undefined4 *)((int)this + 0x1698) = *(undefined4 *)((int)this + 0x2698);
      return 0;
    }
    if ((param_2 == 0x201) || (param_2 == 0x204)) {
      SetFocus(param_1);
      return 0;
    }
    if ((param_2 == 0x20a) && (*(int *)((int)this + 0x230c) != 0)) {
      *(int *)((int)this + 0x2310) = (int)(short)(param_3 >> 0x10) + *(int *)((int)this + 0x2310);
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
  LVar4 = DefWindowProcA(param_1,param_2,param_3,(LPARAM)param_4);
  return LVar4;
}

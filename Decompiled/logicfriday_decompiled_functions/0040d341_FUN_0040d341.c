/* 0040d341 FUN_0040d341 */

undefined4 __cdecl FUN_0040d341(LONG param_1,LONG param_2)

{
  POINT pt;
  bool bVar1;
  BOOL BVar2;
  undefined4 uVar3;
  LRESULT LVar4;
  int iVar5;
  int local_9c;
  int local_98;
  int local_94;
  int local_90;
  int local_8c;
  int local_88;
  int local_84;
  int local_80;
  int local_7c;
  int local_78;
  int local_74;
  int local_60;
  int local_5c;
  HMENU local_58;
  int local_54;
  int local_50;
  int local_4c;
  int local_48;
  int local_44;
  int local_40;
  int local_3c;
  int local_38;
  int local_34;
  tagPOINT local_30;
  LRESULT local_28;
  HWND local_24;
  uint local_20;
  tagRECT local_1c;
  HMENU local_c;
  int local_8;
  
  local_30.x = param_1;
  local_30.y = param_2;
  local_58 = (HMENU)0x0;
  GetClientRect(DAT_00452aac,&local_1c);
  ScreenToClient(DAT_00452aac,&local_30);
  pt.y = local_30.y;
  pt.x = local_30.x;
  BVar2 = PtInRect(&local_1c,pt);
  if (BVar2 == 0) {
    uVar3 = 0;
  }
  else if ((((DAT_00452eec == 0) && (DAT_00452e98 == 0)) && (DAT_00452e94 == 0)) &&
          (DAT_00452e90 == 0)) {
    uVar3 = 0;
  }
  else if ((DAT_00452e94 == 0) || (LVar4 = SendMessageA(DAT_00452a98,0x111,0x801e,0), LVar4 != 0)) {
    local_20 = 0;
    if (((DAT_00452e7c == 0) && ((DAT_00452eb4 == 0 && (DAT_00452eb0 == 0)))) && (DAT_00452ed8 == 0)
       ) {
      bVar1 = false;
    }
    else {
      bVar1 = true;
    }
    if (((DAT_00452e98 == 0) && (DAT_00452e90 == 0)) && (DAT_00452e94 == 0)) {
      local_74 = 0;
    }
    else {
      local_74 = 1;
    }
    local_5c = local_74;
    local_38 = 0;
    local_8 = 0;
    local_34 = 0;
    local_3c = 0;
    local_54 = 0;
    local_50 = 0;
    local_4c = 0;
    local_48 = 0;
    local_44 = 0;
    local_40 = 0;
    local_28 = FUN_0040f186(&DAT_00452ad0,&local_40);
    if ((local_28 != 0) && (local_40 != 0)) {
      local_38 = *(int *)(local_40 + 0x250);
      local_8 = *(int *)(local_40 + 0x1650);
      local_34 = *(int *)(local_40 + 0x16b4);
      if (local_8 != 0) {
        SendMessageA(DAT_00452a2c,0x111,0x8016,(LPARAM)&local_20);
      }
    }
    if (DAT_00452e94 != 0) {
      SendMessageA(DAT_00452a98,0x111,0x8017,(LPARAM)&local_60);
      SendMessageA(DAT_00452a98,0x111,0x801f,(LPARAM)&local_3c);
    }
    if (DAT_00452ea0 != 0) {
      FUN_00439b85(&DAT_00453e28,&local_54);
    }
    ClientToScreen(DAT_00452aac,&local_30);
    local_24 = GetFocus();
    if (local_24 == DAT_00452ab4) {
      local_58 = LoadMenuA(DAT_00452914,"MENU_TTOUT");
      if (local_58 == (HMENU)0x0) {
        return 0;
      }
      FUN_0040ca01(local_58,0x14c,(uint)(local_28 == 1));
    }
    else if ((local_24 == DAT_00452ab0) && (DAT_00452e98 == 0)) {
      local_58 = LoadMenuA(DAT_00452914,"MENU_LEOUT");
      if (local_58 == (HMENU)0x0) {
        return 0;
      }
      if ((bVar1) || (local_48 == 0)) {
        local_78 = 0;
      }
      else {
        local_78 = 1;
      }
      FUN_0040ca01(local_58,0xad,local_78);
      if ((((local_28 != 1) || (bVar1)) || (local_38 != 0)) || (local_5c != 0)) {
        local_7c = 0;
      }
      else {
        local_7c = 1;
      }
      FUN_0040ca01(local_58,0x14d,local_7c);
      if (((local_28 != 1) || (bVar1)) || ((local_38 != 0 || (local_5c != 0)))) {
        local_80 = 0;
      }
      else {
        local_80 = 1;
      }
      FUN_0040ca01(local_58,0x14e,local_80);
      if (((local_28 != 1) || (bVar1)) || ((local_38 != 0 || (local_5c != 0)))) {
        local_84 = 0;
      }
      else {
        local_84 = 1;
      }
      FUN_0040ca01(local_58,0x4b0,local_84);
    }
    else if (local_24 == DAT_00452a2c) {
      local_58 = LoadMenuA(DAT_00452914,"MENU_DIAGOUT");
      if (local_58 == (HMENU)0x0) {
        return 0;
      }
      if ((local_28 == 0) || (*(int *)(local_40 + 0x16b8) == 0)) {
        CheckMenuItem(local_58,0x152,0);
      }
      else {
        CheckMenuItem(local_58,0x152,8);
      }
      if ((bVar1) || ((local_20 & 0xffff) == 0)) {
        local_88 = 0;
      }
      else {
        local_88 = 1;
      }
      FUN_0040ca01(local_58,0x153,local_88);
      if ((bVar1) || (local_20 >> 0x10 == 0)) {
        local_8c = 0;
      }
      else {
        local_8c = 1;
      }
      FUN_0040ca01(local_58,0x154,local_8c);
      if ((bVar1) || (local_20 >> 0x10 == 0)) {
        local_90 = 0;
      }
      else {
        local_90 = 1;
      }
      FUN_0040ca01(local_58,0x155,local_90);
      FUN_0040ca01(local_58,0x151,local_8);
      if (((local_28 == 1) && (local_8 != 0)) && (local_5c == 0)) {
        local_94 = 1;
      }
      else {
        local_94 = 0;
      }
      FUN_0040ca01(local_58,0x4b1,local_94);
      if (((local_28 == 1) && (local_34 != 0)) && (local_5c == 0)) {
        local_98 = 1;
      }
      else {
        local_98 = 0;
      }
      FUN_0040ca01(local_58,0x15a,local_98);
      if (((local_28 != 1) || (bVar1)) || ((local_5c != 0 || (local_8 == 0)))) {
        local_9c = 0;
      }
      else {
        local_9c = 1;
      }
      FUN_0040ca01(local_58,0x152,local_9c);
    }
    else if ((local_24 == DAT_00452ab8) && (DAT_00452e90 != 0)) {
      local_58 = LoadMenuA(DAT_00452914,"MENU_TTIN");
      if (local_58 == (HMENU)0x0) {
        return 0;
      }
      iVar5 = FUN_0041a2e8(0x452b30);
      if (iVar5 == 0) {
        FUN_0040ca01(local_58,0xe0,0);
        FUN_0040ca01(local_58,0xe1,0);
        FUN_0040ca01(local_58,0xe2,0);
        FUN_0040ca01(local_58,0xe4,0);
      }
    }
    else if ((local_24 == DAT_00452ab0) && (DAT_00452e98 != 0)) {
      local_58 = LoadMenuA(DAT_00452914,"MENU_LEIN");
      if (local_58 == (HMENU)0x0) {
        return 0;
      }
      FUN_0040ca01(local_58,0xb3,local_54);
      FUN_0040ca01(local_58,0xb4,local_50);
      FUN_0040ca01(local_58,0xcb,local_48);
      FUN_0040ca01(local_58,0xad,local_48);
      FUN_0040ca01(local_58,0xae,local_44);
      FUN_0040ca01(local_58,0xaf,local_48);
      FUN_0040ca01(local_58,0xb0,local_4c);
    }
    else if ((local_24 == DAT_00452a98) && (DAT_00452e94 != 0)) {
      local_58 = LoadMenuA(DAT_00452914,"MENU_DIAGIN");
      if (local_58 == (HMENU)0x0) {
        return 0;
      }
      FUN_0040ca01(local_58,0x151,local_3c);
      FUN_0040ca01(local_58,0x4b2,local_60);
      FUN_0040ca01(local_58,0x114,local_3c);
    }
    local_c = GetSubMenu(local_58,0);
    TrackPopupMenu(local_c,2,local_30.x,local_30.y,0,DAT_00452aac,(RECT *)0x0);
    DestroyMenu(local_58);
    uVar3 = 1;
  }
  else {
    uVar3 = 0;
  }
  return uVar3;
}

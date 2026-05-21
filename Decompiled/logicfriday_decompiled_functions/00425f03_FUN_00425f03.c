/* 00425f03 FUN_00425f03 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void FUN_00425f03(HDC param_1,int *param_2,int param_3,int param_4,int param_5)

{
  HGDIOBJ pvVar1;
  size_t sVar2;
  int iVar3;
  uint unaff_retaddr;
  tagSIZE *ptVar4;
  int iVar5;
  LOGFONTA local_104;
  HFONT local_c8;
  HGDIOBJ local_c4;
  LOGFONTA local_c0;
  uint local_84;
  HFONT local_80;
  int local_7c;
  int local_78;
  int local_74 [12];
  tagSIZE local_44;
  int local_3c;
  POINT local_38;
  int local_30;
  int local_2c;
  int local_28;
  int local_24;
  int local_20;
  int local_1c;
  int local_18;
  int local_14;
  int local_10;
  int local_c;
  HGDIOBJ local_8;
  
  local_84 = DAT_00451a00 ^ unaff_retaddr;
  local_3c = 0;
  local_74[0] = 4;
  local_74[1] = 4;
  local_74[2] = 0;
  local_74[3] = 0;
  local_74[4] = 4;
  local_74[5] = 7;
  local_74[6] = 4;
  local_74[7] = 0;
  local_74[8] = 4;
  local_74[9] = 7;
  local_74[10] = 7;
  local_74[0xb] = 4;
  local_c = 0;
  local_18 = 0;
  local_14 = 0;
  pvVar1 = GetStockObject(5);
  local_8 = SelectObject(param_1,pvVar1);
  SetBkMode(param_1,1);
  param_2[0x30] = param_3;
  param_2[0x31] = param_4;
  switch(*param_2) {
  case 0:
    MoveToEx(param_1,param_3 + 0xf,param_4 + 5,(LPPOINT)0x0);
    LineTo(param_1,param_3 + 0xf,param_4 + 0x2d);
    LineTo(param_1,param_3 + 0x37,param_4 + 0x19);
    LineTo(param_1,param_3 + 0xf,param_4 + 5);
    Ellipse(param_1,param_3 + 0x37,param_4 + 0x15,param_3 + 0x40,param_4 + 0x1e);
    MoveToEx(param_1,param_3 + 100,param_4 + 0x19,(LPPOINT)0x0);
    LineTo(param_1,param_3 + 0x3f,param_4 + 0x19);
    param_2[0x2b] = param_3 + 100;
    param_2[0x2c] = param_4 + 0x19;
    MoveToEx(param_1,param_3,param_4 + 0x19,(LPPOINT)0x0);
    LineTo(param_1,param_3 + 0xf,param_4 + 0x19);
    param_2[0x1b] = param_3;
    param_2[0x1c] = param_4 + 0x19;
    param_2[0x23] = param_3 + 0xf;
    param_2[0x24] = param_4 + 0x19;
    SetRect((LPRECT)(param_2 + 0x32),param_3,param_4 + 3,param_3 + 0x65,param_4 + 0x30);
    if (param_5 != 0) {
      sVar2 = _strlen((char *)(param_2 + 0x14));
      TextOutA(param_1,param_3 + 0x1e,param_4 + 0x32,(LPCSTR)(param_2 + 0x14),sVar2);
    }
    break;
  case 1:
    local_c = 1;
  case 6:
    MoveToEx(param_1,param_3 + 0x37,param_4,(LPPOINT)0x0);
    LineTo(param_1,param_3 + 0xf,param_4);
    LineTo(param_1,param_3 + 0xf,param_4 + 0x32);
    LineTo(param_1,param_3 + 0x37,param_4 + 0x32);
    Arc(param_1,param_3 + 0x1e,param_4,param_3 + 0x50,param_4 + 0x33,param_3 + 0x37,param_4 + 0x33,
        param_3 + 0x37,param_4);
    if (local_c == 0) {
      MoveToEx(param_1,param_3 + 100,param_4 + 0x19,(LPPOINT)0x0);
      LineTo(param_1,param_3 + 0x4f,param_4 + 0x19);
      param_2[0x2b] = param_3 + 100;
    }
    else {
      Ellipse(param_1,param_3 + 0x4f,param_4 + 0x15,param_3 + 0x58,param_4 + 0x1e);
      MoveToEx(param_1,param_3 + 100,param_4 + 0x19,(LPPOINT)0x0);
      LineTo(param_1,param_3 + 0x57,param_4 + 0x19);
      param_2[0x2b] = param_3 + 100;
    }
    param_2[0x2c] = param_4 + 0x19;
    if (param_2[6] == 2) {
      local_78 = 0x1e;
    }
    else if (param_2[6] == 3) {
      local_78 = 0xf;
    }
    else if (param_2[6] == 4) {
      local_78 = 10;
    }
    for (local_10 = 0; local_10 < param_2[6]; local_10 = local_10 + 1) {
      MoveToEx(param_1,param_3,param_4 + 10 + local_10 * local_78,(LPPOINT)0x0);
      LineTo(param_1,param_3 + 0xf,param_4 + 10 + local_10 * local_78);
    }
    for (local_10 = 0; local_10 < param_2[6]; local_10 = local_10 + 1) {
      param_2[local_10 * 2 + 0x1b] = param_3;
      param_2[local_10 * 2 + 0x1c] = param_4 + 10 + local_10 * local_78;
      param_2[local_10 * 2 + 0x23] = param_3 + 0xf;
      param_2[local_10 * 2 + 0x24] = param_4 + 10 + local_10 * local_78;
    }
    SetRect((LPRECT)(param_2 + 0x32),param_3,param_4 + -2,param_3 + 0x65,param_4 + 0x35);
    if (param_5 != 0) {
      sVar2 = _strlen((char *)(param_2 + 0x14));
      TextOutA(param_1,param_3 + 0x1e,param_4 + 0x37,(LPCSTR)(param_2 + 0x14),sVar2);
    }
    break;
  case 2:
    local_18 = 1;
  case 3:
    if (local_18 == 0) {
      local_14 = 1;
      local_3c = -7;
    }
    else {
      local_3c = 0;
    }
  case 7:
    MoveToEx(param_1,param_3 + 0x23,param_4,(LPPOINT)0x0);
    LineTo(param_1,param_3 + 0xf,param_4);
    MoveToEx(param_1,param_3 + 0xf,param_4 + 0x32,(LPPOINT)0x0);
    LineTo(param_1,param_3 + 0x23,param_4 + 0x32);
    Arc(param_1,param_3 + -0x4d,param_4 + -0x19,param_3 + 0x17,param_4 + 0x4b,param_3 + 0xf,
        param_4 + 0x32,param_3 + 0xf,param_4);
    if (local_14 != 0) {
      Arc(param_1,param_3 + -0x4d + local_3c,param_4 + -0x19,param_3 + 0x17 + local_3c,
          param_4 + 0x4b,param_3 + 0xf + local_3c,param_4 + 0x32,param_3 + 0xf + local_3c,param_4);
    }
    local_38.x = param_3 + 0x23;
    local_38.y = param_4;
    local_30 = param_3 + 0x37;
    local_2c = param_4 + -1;
    local_28 = param_3 + 0x4b;
    local_24 = param_4 + 0x12;
    local_20 = param_3 + 0x50;
    local_1c = param_4 + 0x19;
    PolyBezier(param_1,&local_38,4);
    local_38.x = param_3 + 0x23;
    local_38.y = param_4 + 0x32;
    local_30 = param_3 + 0x32;
    local_2c = param_4 + 0x33;
    local_28 = param_3 + 0x4b;
    local_24 = param_4 + 0x20;
    local_20 = param_3 + 0x50;
    local_1c = param_4 + 0x19;
    PolyBezier(param_1,&local_38,4);
    if (local_18 == 0) {
      MoveToEx(param_1,param_3 + 100,param_4 + 0x19,(LPPOINT)0x0);
      LineTo(param_1,param_3 + 0x50,param_4 + 0x19);
      param_2[0x2b] = param_3 + 100;
    }
    else {
      Ellipse(param_1,param_3 + 0x50,param_4 + 0x15,param_3 + 0x59,param_4 + 0x1e);
      MoveToEx(param_1,param_3 + 100,param_4 + 0x19,(LPPOINT)0x0);
      LineTo(param_1,param_3 + 0x58,param_4 + 0x19);
      param_2[0x2b] = param_3 + 100;
    }
    param_2[0x2c] = param_4 + 0x19;
    if (param_2[6] == 2) {
      local_78 = 0x1e;
    }
    else if (param_2[6] == 3) {
      local_78 = 0xf;
    }
    else if (param_2[6] == 4) {
      local_78 = 10;
    }
    local_7c = param_2[6];
    for (local_10 = 0; local_10 < local_7c; local_10 = local_10 + 1) {
      MoveToEx(param_1,param_3,param_4 + 10 + local_10 * local_78,(LPPOINT)0x0);
      LineTo(param_1,param_3 + 0xf + local_3c + local_74[(local_7c + -2) * 4 + local_10],
             param_4 + 10 + local_10 * local_78);
    }
    for (local_10 = 0; local_10 < param_2[6]; local_10 = local_10 + 1) {
      param_2[local_10 * 2 + 0x1b] = param_3;
      param_2[local_10 * 2 + 0x1c] = param_4 + 10 + local_10 * local_78;
      param_2[local_10 * 2 + 0x23] =
           param_3 + 0xf + local_3c + local_74[(local_7c + -2) * 4 + local_10];
      param_2[local_10 * 2 + 0x24] = param_4 + 10 + local_10 * local_78;
    }
    SetRect((LPRECT)(param_2 + 0x32),param_3,param_4 + -2,param_3 + 0x65,param_4 + 0x35);
    if (param_5 != 0) {
      sVar2 = _strlen((char *)(param_2 + 0x14));
      TextOutA(param_1,param_3 + 0x1e,param_4 + 0x37,(LPCSTR)(param_2 + 0x14),sVar2);
    }
    break;
  case 5:
    _memset(&local_c0,0,0x3c);
    iVar5 = 0x48;
    iVar3 = GetDeviceCaps(param_1,0x5a);
    local_c0.lfHeight = MulDiv(8,iVar3,iVar5);
    local_c0.lfHeight = -local_c0.lfHeight;
    local_c0.lfCharSet = '\0';
    local_c0.lfWeight = 100;
    FUN_0043ed39(local_c0.lfFaceName,(byte *)"COURIER NEW");
    local_80 = CreateFontIndirectA(&local_c0);
    Rectangle(param_1,param_3 + 0xf,param_4,param_3 + 0x41,param_4 + 0x32);
    MoveToEx(param_1,param_3,param_4 + 10,(LPPOINT)0x0);
    LineTo(param_1,param_3 + 0xf,param_4 + 10);
    param_2[0x1b] = param_3;
    param_2[0x1c] = param_4 + 10;
    param_2[0x23] = param_3 + 0xf;
    param_2[0x24] = param_4 + 10;
    MoveToEx(param_1,param_3,param_4 + 0x19,(LPPOINT)0x0);
    LineTo(param_1,param_3 + 0xf,param_4 + 0x19);
    param_2[0x1d] = param_3;
    param_2[0x1e] = param_4 + 0x19;
    param_2[0x25] = param_3 + 0xf;
    param_2[0x26] = param_4 + 0x19;
    MoveToEx(param_1,param_3 + 100,param_4 + 0x19,(LPPOINT)0x0);
    LineTo(param_1,param_3 + 0x41,param_4 + 0x19);
    param_2[0x2b] = param_3 + 100;
    param_2[0x2c] = param_4 + 0x19;
    MoveToEx(param_1,param_3,param_4 + 0x28,(LPPOINT)0x0);
    LineTo(param_1,param_3 + 0xf,param_4 + 0x28);
    param_2[0x1f] = param_3;
    param_2[0x20] = param_4 + 0x28;
    param_2[0x27] = param_3 + 0xf;
    param_2[0x28] = param_4 + 0x28;
    if (param_5 != 0) {
      sVar2 = _strlen((char *)(param_2 + 0x14));
      TextOutA(param_1,param_3 + 0x14,param_4 + 0x37,(LPCSTR)(param_2 + 0x14),sVar2);
    }
    local_c4 = SelectObject(param_1,local_80);
    TextOutA(param_1,param_3 + 0x11,param_4 + 5,"D0",2);
    TextOutA(param_1,param_3 + 0x11,param_4 + 0x14,"D1",2);
    TextOutA(param_1,param_3 + 0x11,param_4 + 0x23,"S",1);
    TextOutA(param_1,param_3 + 0x29,param_4 + 0x12,"OUT",3);
    SetRect((LPRECT)(param_2 + 0x32),param_3,param_4 + -2,param_3 + 0x65,param_4 + 0x35);
    pvVar1 = SelectObject(param_1,local_c4);
    DeleteObject(pvVar1);
    break;
  case 8:
    iVar3 = param_3 + 0x14;
    MoveToEx(param_1,param_3 + 0x28,param_4 + 0x19,(LPPOINT)0x0);
    LineTo(param_1,iVar3,param_4 + 0x19);
    param_2[0x2b] = param_3 + 0x28;
    param_2[0x2c] = param_4 + 0x19;
    MoveToEx(param_1,iVar3,param_4 + 0xf,(LPPOINT)0x0);
    LineTo(param_1,iVar3,param_4 + 0x23);
    if (param_5 == 0) {
      SetRect((LPRECT)(param_2 + 0x32),param_3 + 0x13,param_4 + 0xd,param_3 + 0x29,param_4 + 0x26);
    }
    else {
      SetTextAlign(param_1,2);
      sVar2 = _strlen((char *)(param_2 + 0x14));
      TextOutA(param_1,param_3 + 0x10,param_4 + 0xd,(LPCSTR)(param_2 + 0x14),sVar2);
      ptVar4 = &local_44;
      sVar2 = _strlen((char *)(param_2 + 0x14));
      GetTextExtentPoint32A(param_1,(LPCSTR)(param_2 + 0x14),sVar2,ptVar4);
      SetRect((LPRECT)(param_2 + 0x32),(param_3 + 0xd) - local_44.cx,param_4 + 0xd,param_3 + 0x29,
              param_4 + 0x26);
      SetPixel(param_1,param_2[0x32] + -1,param_2[0x33],0xffffff);
      SetTextAlign(param_1,0);
    }
    break;
  case 9:
    MoveToEx(param_1,param_3,param_4 + 0x19,(LPPOINT)0x0);
    LineTo(param_1,param_3 + 0x14,param_4 + 0x19);
    param_2[0x1b] = param_3;
    param_2[0x1c] = param_4 + 0x19;
    param_2[0x23] = param_3 + 0x14;
    param_2[0x24] = param_4 + 0x19;
    MoveToEx(param_1,param_3 + 0x14,param_4 + 0xf,(LPPOINT)0x0);
    LineTo(param_1,param_3 + 0x14,param_4 + 0x23);
    if (param_5 == 0) {
      SetRect((LPRECT)(param_2 + 0x32),param_3,param_4 + 0xf,param_3 + 0x17,param_4 + 0x26);
    }
    else {
      sVar2 = _strlen((char *)(param_2 + 0x14));
      TextOutA(param_1,param_3 + 0x18,param_4 + 0xd,(LPCSTR)(param_2 + 0x14),sVar2);
      ptVar4 = &local_44;
      sVar2 = _strlen((char *)(param_2 + 0x14));
      GetTextExtentPoint32A(param_1,(LPCSTR)(param_2 + 0x14),sVar2,ptVar4);
      SetRect((LPRECT)(param_2 + 0x32),param_3 + -1,param_4 + 0xd,param_3 + 0x1b + local_44.cx,
              param_4 + 0x26);
    }
    break;
  case 10:
  case 0xb:
    MoveToEx(param_1,param_3,param_4,(LPPOINT)0x0);
    LineTo(param_1,param_3 + -0xf,param_4);
    MoveToEx(param_1,param_3 + -0xf,param_4 + -5,(LPPOINT)0x0);
    LineTo(param_1,param_3 + -0xf,param_4 + 5);
    param_2[0x2b] = param_3;
    param_2[0x2c] = param_4;
    _memset(&local_104,0,0x3c);
    iVar5 = 0x48;
    iVar3 = GetDeviceCaps(param_1,0x5a);
    local_104.lfHeight = MulDiv(0xc,iVar3,iVar5);
    local_104.lfHeight = -local_104.lfHeight;
    local_104.lfCharSet = '\0';
    local_104.lfWeight = 100;
    FUN_0043ed39(local_104.lfFaceName,(byte *)"COURIER NEW");
    local_c8 = CreateFontIndirectA(&local_104);
    pvVar1 = SelectObject(param_1,local_c8);
    SetTextAlign(param_1,0);
    if (*param_2 == 0xb) {
      TextOutA(param_1,param_3 + -0x1b,param_4 + -9,"0",1);
    }
    else {
      TextOutA(param_1,param_3 + -0x1b,param_4 + -9,"1",1);
    }
    pvVar1 = SelectObject(param_1,pvVar1);
    DeleteObject(pvVar1);
    SetRect((LPRECT)(param_2 + 0x32),param_3 + -0x1b,param_4 + -9,param_3 + 1,param_4 + 9);
  }
  SelectObject(param_1,local_8);
  return;
}

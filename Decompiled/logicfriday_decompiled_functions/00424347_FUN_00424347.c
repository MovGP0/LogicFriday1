/* 00424347 FUN_00424347 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 FUN_00424347(void)

{
  void *pvVar1;
  undefined4 uVar2;
  HDC pHVar3;
  int iVar4;
  HFONT pHVar5;
  HGDIOBJ pvVar6;
  HENHMETAFILE pHVar7;
  int iVar8;
  HPEN pHVar9;
  undefined4 extraout_ECX;
  int iVar10;
  int unaff_EBP;
  int iVar11;
  
  FUN_0043f30c();
  *(uint *)(unaff_EBP + -0x10) = DAT_00451a00 ^ *(uint *)(unaff_EBP + 4);
  *(undefined4 *)(unaff_EBP + -0x410) = extraout_ECX;
  *(undefined4 *)(unaff_EBP + -0x3ec) = 0x96;
  *(undefined4 *)(unaff_EBP + -0x118) = 0x96;
  FUN_004175df(unaff_EBP + -0x32c);
  *(undefined4 *)(unaff_EBP + -4) = 0;
  *(undefined4 *)(unaff_EBP + -0x11c) = 0;
  while (*(int *)(unaff_EBP + -0x11c) < *(int *)(*(int *)(unaff_EBP + -0x410) + 0x16c8)) {
    *(undefined4 *)(unaff_EBP + -0x3fc) =
         *(undefined4 *)
          (*(int *)(*(int *)(unaff_EBP + -0x410) + 0x16d0) + *(int *)(unaff_EBP + -0x11c) * 4);
    *(undefined4 *)(unaff_EBP + -0x3f8) = *(undefined4 *)(unaff_EBP + -0x3fc);
    if (*(int *)(unaff_EBP + -0x3f8) == 0) {
      *(undefined4 *)(unaff_EBP + -0x414) = 0;
    }
    else {
      pvVar1 = FUN_0041d91b(*(void **)(unaff_EBP + -0x3f8),1);
      *(void **)(unaff_EBP + -0x414) = pvVar1;
    }
    *(int *)(unaff_EBP + -0x11c) = *(int *)(unaff_EBP + -0x11c) + 1;
  }
  *(undefined4 *)(*(int *)(unaff_EBP + -0x410) + 0x16c8) = 0;
  uVar2 = FUN_00424b74(*(void **)(unaff_EBP + -0x410));
  *(undefined4 *)(unaff_EBP + -0x330) = uVar2;
  if (*(int *)(unaff_EBP + -0x330) == 0) {
    *(undefined4 *)(unaff_EBP + -0x11c) = 0;
    while (*(int *)(unaff_EBP + -0x11c) < *(int *)(*(int *)(unaff_EBP + -0x410) + 0x2670)) {
      *(undefined4 *)
       (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x410) + 0x2678) + *(int *)(unaff_EBP + -0x11c) * 4)
       + 0x14) = 0x80;
      *(int *)(unaff_EBP + -0x11c) = *(int *)(unaff_EBP + -0x11c) + 1;
    }
    *(undefined4 *)(unaff_EBP + -0x3f4) = 0;
    while (*(int *)(unaff_EBP + -0x3f4) < *(int *)(*(int *)(unaff_EBP + -0x410) + 0x2674)) {
      *(undefined4 *)
       (**(int **)(*(int *)(unaff_EBP + -0x410) + 0x2678) + 8 + *(int *)(unaff_EBP + -0x3f4) * 0x48)
           = 0x80;
      *(int *)(unaff_EBP + -0x3f4) = *(int *)(unaff_EBP + -0x3f4) + 1;
    }
    _memset((void *)(unaff_EBP + -0x370),0,0x3c);
    if (*(int *)(*(int *)(unaff_EBP + -0x410) + 0x16b0) != 0) {
      DeleteEnhMetaFile(*(HENHMETAFILE *)(*(int *)(unaff_EBP + -0x410) + 0x16b0));
    }
    pHVar3 = CreateEnhMetaFileA((HDC)0x0,(LPCSTR)0x0,(RECT *)0x0,(LPCSTR)0x0);
    *(HDC *)(unaff_EBP + -0x228) = pHVar3;
    *(undefined4 *)(*(int *)(unaff_EBP + -0x410) + 0x2680) = 1;
    iVar11 = 0x48;
    iVar4 = GetDeviceCaps(*(HDC *)(unaff_EBP + -0x228),0x5a);
    iVar4 = MulDiv(0xc,iVar4,iVar11);
    *(int *)(unaff_EBP + -0x370) = -iVar4;
    *(undefined1 *)(unaff_EBP + -0x359) = 0;
    *(undefined4 *)(unaff_EBP + -0x360) = 100;
    FUN_0043ed39((char *)(unaff_EBP + -0x354),(byte *)"COURIER NEW");
    pHVar5 = CreateFontIndirectA((LOGFONTA *)(unaff_EBP + -0x370));
    *(HFONT *)(unaff_EBP + -0x334) = pHVar5;
    pvVar6 = SelectObject(*(HDC *)(unaff_EBP + -0x228),*(HGDIOBJ *)(unaff_EBP + -0x334));
    *(HGDIOBJ *)(unaff_EBP + -1000) = pvVar6;
    SetTextColor(*(HDC *)(unaff_EBP + -0x228),0);
    SetBkColor(*(HDC *)(unaff_EBP + -0x228),0xffffff);
    SetBkMode(*(HDC *)(unaff_EBP + -0x228),1);
    FUN_00425b8e(*(void **)(unaff_EBP + -0x410),*(HDC *)(unaff_EBP + -0x228));
    FUN_00426f50();
    pHVar7 = CloseEnhMetaFile(*(HDC *)(unaff_EBP + -0x228));
    *(HENHMETAFILE *)(*(int *)(unaff_EBP + -0x410) + 0x16b0) = pHVar7;
    DeleteEnhMetaFile(*(HENHMETAFILE *)(*(int *)(unaff_EBP + -0x410) + 0x16b0));
    FUN_00425991(*(int *)(unaff_EBP + -0x410));
    *(undefined4 *)(unaff_EBP + -0x11c) = 0;
    while (*(int *)(unaff_EBP + -0x11c) < *(int *)(*(int *)(unaff_EBP + -0x410) + 0x1650)) {
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x410) + 0x3a4) + 0xb4 + *(int *)(unaff_EBP + -0x11c) * 0xfc)
           = 0;
      _memset((void *)(*(int *)(*(int *)(unaff_EBP + -0x410) + 0x3a4) + 0x6c +
                      *(int *)(unaff_EBP + -0x11c) * 0xfc),0,0x20);
      iVar8 = *(int *)(unaff_EBP + -0x11c) * 0xfc;
      iVar4 = *(int *)(*(int *)(unaff_EBP + -0x410) + 0x3a4);
      uVar2 = *(undefined4 *)(iVar4 + 0x70 + iVar8);
      iVar10 = *(int *)(unaff_EBP + -0x11c) * 0xfc;
      iVar11 = *(int *)(*(int *)(unaff_EBP + -0x410) + 0x3a4);
      *(undefined4 *)(iVar11 + 0xac + iVar10) = *(undefined4 *)(iVar4 + 0x6c + iVar8);
      *(undefined4 *)(iVar11 + 0xb0 + iVar10) = uVar2;
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x410) + 0x3a4) + 0xb8 + *(int *)(unaff_EBP + -0x11c) * 0xfc)
           = 0;
      *(int *)(unaff_EBP + -0x11c) = *(int *)(unaff_EBP + -0x11c) + 1;
    }
    *(undefined4 *)(unaff_EBP + -0x11c) = 0;
    while (*(int *)(unaff_EBP + -0x11c) < *(int *)(*(int *)(unaff_EBP + -0x410) + 0x16c8)) {
      *(undefined4 *)(unaff_EBP + -0x408) =
           *(undefined4 *)
            (*(int *)(*(int *)(unaff_EBP + -0x410) + 0x16d0) + *(int *)(unaff_EBP + -0x11c) * 4);
      *(undefined4 *)(unaff_EBP + -0x404) = *(undefined4 *)(unaff_EBP + -0x408);
      if (*(int *)(unaff_EBP + -0x404) == 0) {
        *(undefined4 *)(unaff_EBP + -0x418) = 0;
      }
      else {
        pvVar1 = FUN_0041d91b(*(void **)(unaff_EBP + -0x404),1);
        *(void **)(unaff_EBP + -0x418) = pvVar1;
      }
      *(int *)(unaff_EBP + -0x11c) = *(int *)(unaff_EBP + -0x11c) + 1;
    }
    *(undefined4 *)(*(int *)(unaff_EBP + -0x410) + 0x16c8) = 0;
    *(undefined4 *)(*(int *)(unaff_EBP + -0x410) + 0x2680) = 0;
    FUN_0043ebd0((uint *)(unaff_EBP + -0x224),(uint *)(*(int *)(unaff_EBP + -0x410) + 0xd0));
    if (1 < *(uint *)(*(int *)(unaff_EBP + -0x410) + 200)) {
      FUN_0043ebe0((uint *)(unaff_EBP + -0x224),(uint *)&DAT_0044ac88);
      FUN_0043ebe0((uint *)(unaff_EBP + -0x224),
                   (uint *)(*(int *)(unaff_EBP + -0x410) + 0xd0 +
                           (*(int *)(*(int *)(unaff_EBP + -0x410) + 200) + -1) * 9));
    }
    FUN_0043ed39((char *)(unaff_EBP + -0x114),(byte *)"Logic Friday Diagram");
    pHVar3 = CreateEnhMetaFileA((HDC)0x0,(LPCSTR)0x0,(RECT *)0x0,(LPCSTR)(unaff_EBP + -0x114));
    *(HDC *)(unaff_EBP + -0x228) = pHVar3;
    pvVar6 = SelectObject(*(HDC *)(unaff_EBP + -0x228),*(HGDIOBJ *)(unaff_EBP + -0x334));
    *(HGDIOBJ *)(unaff_EBP + -1000) = pvVar6;
    pHVar9 = CreatePen(0,1,0);
    *(HPEN *)(unaff_EBP + -0x120) = pHVar9;
    pvVar6 = SelectObject(*(HDC *)(unaff_EBP + -0x228),*(HGDIOBJ *)(unaff_EBP + -0x120));
    *(HGDIOBJ *)(unaff_EBP + -0x3f0) = pvVar6;
    SetTextColor(*(HDC *)(unaff_EBP + -0x228),0);
    SetBkColor(*(HDC *)(unaff_EBP + -0x228),0xffffff);
    SetBkMode(*(HDC *)(unaff_EBP + -0x228),1);
    FUN_00425b8e(*(void **)(unaff_EBP + -0x410),*(HDC *)(unaff_EBP + -0x228));
    FUN_00426f50();
    FUN_00427c43();
    FUN_00423a22(*(int *)(unaff_EBP + -0x410));
    pvVar6 = SelectObject(*(HDC *)(unaff_EBP + -0x228),*(HGDIOBJ *)(unaff_EBP + -1000));
    DeleteObject(pvVar6);
    pvVar6 = SelectObject(*(HDC *)(unaff_EBP + -0x228),*(HGDIOBJ *)(unaff_EBP + -0x3f0));
    DeleteObject(pvVar6);
    pHVar7 = CloseEnhMetaFile(*(HDC *)(unaff_EBP + -0x228));
    *(HENHMETAFILE *)(*(int *)(unaff_EBP + -0x410) + 0x16b0) = pHVar7;
    *(undefined4 *)(*(int *)(unaff_EBP + -0x410) + 0x1688) =
         *(undefined4 *)(*(int *)(unaff_EBP + -0x410) + 0x16b0);
    GetEnhMetaFileHeader
              (*(HENHMETAFILE *)(*(int *)(unaff_EBP + -0x410) + 0x16b0),0x6c,
               (LPENHMETAHEADER)(unaff_EBP + -0x3e4));
    *(int *)(*(int *)(unaff_EBP + -0x410) + 0x16a8) =
         *(int *)(unaff_EBP + -0x3d0) - *(int *)(unaff_EBP + -0x3d8);
    *(int *)(*(int *)(unaff_EBP + -0x410) + 0x16a4) =
         *(int *)(unaff_EBP + -0x3d4) - *(int *)(unaff_EBP + -0x3dc);
    *(undefined4 *)(*(int *)(unaff_EBP + -0x410) + 0x169c) = 0;
    *(undefined4 *)(*(int *)(unaff_EBP + -0x410) + 0x16a0) = 0;
    *(undefined4 *)(*(int *)(unaff_EBP + -0x410) + 0x267c) = 1;
    if (*(int *)(*(int *)(unaff_EBP + -0x410) + 0x2678) != 0) {
      *(undefined4 *)(unaff_EBP + -0x11c) = 0;
      while (*(int *)(unaff_EBP + -0x11c) < *(int *)(*(int *)(unaff_EBP + -0x410) + 0x2670)) {
        _free(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x410) + 0x2678) +
                        *(int *)(unaff_EBP + -0x11c) * 4));
        *(int *)(unaff_EBP + -0x11c) = *(int *)(unaff_EBP + -0x11c) + 1;
      }
      _free(*(void **)(*(int *)(unaff_EBP + -0x410) + 0x2678));
      *(undefined4 *)(*(int *)(unaff_EBP + -0x410) + 0x2678) = 0;
    }
    if ((DAT_00452ef4 != 0) && (*(int *)(*(int *)(unaff_EBP + -0x410) + 0x2680) == 0)) {
      *(undefined4 *)(unaff_EBP + -0x11c) = *(undefined4 *)(*(int *)(unaff_EBP + -0x410) + 0x1654);
      while (*(int *)(unaff_EBP + -0x11c) < *(int *)(*(int *)(unaff_EBP + -0x410) + 0x1650)) {
        if (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x410) + 0x3a4) + 0x48 +
                    *(int *)(unaff_EBP + -0x11c) * 0xfc) == 0) {
          *(undefined4 *)(unaff_EBP + -0x3f4) = 0;
          while (*(int *)(unaff_EBP + -0x3f4) <
                 *(int *)(*(int *)(*(int *)(unaff_EBP + -0x410) + 0x3a4) + 0x18 +
                         *(int *)(unaff_EBP + -0x11c) * 0xfc)) {
            *(int *)(unaff_EBP + -0x3f4) = *(int *)(unaff_EBP + -0x3f4) + 1;
          }
        }
        *(int *)(unaff_EBP + -0x11c) = *(int *)(unaff_EBP + -0x11c) + 1;
      }
      *(undefined4 *)(unaff_EBP + -0x11c) = 0;
      while (*(int *)(unaff_EBP + -0x11c) < *(int *)(*(int *)(unaff_EBP + -0x410) + 0x16c8)) {
        FUN_0043e34a(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x410) + 0x16d0) +
                             *(int *)(unaff_EBP + -0x11c) * 4));
        *(int *)(unaff_EBP + -0x11c) = *(int *)(unaff_EBP + -0x11c) + 1;
      }
    }
    *(undefined4 *)(*(int *)(unaff_EBP + -0x410) + 0x16b4) = 0;
    *(undefined4 *)(unaff_EBP + -0x40c) = 0;
    *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
    FUN_0043961a();
    uVar2 = *(undefined4 *)(unaff_EBP + -0x40c);
  }
  else {
    *(undefined4 *)(unaff_EBP + -0x400) = *(undefined4 *)(unaff_EBP + -0x330);
    *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
    FUN_0043961a();
    uVar2 = *(undefined4 *)(unaff_EBP + -0x400);
  }
  ExceptionList = *(void **)(unaff_EBP + -0xc);
  return uVar2;
}

/* 00439133 FUN_00439133 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 FUN_00439133(void)

{
  HDC pHVar1;
  int iVar2;
  HFONT pHVar3;
  HGDIOBJ pvVar4;
  HENHMETAFILE pHVar5;
  undefined4 extraout_ECX;
  int unaff_EBP;
  int nDenominator;
  
  FUN_0043f30c();
  *(uint *)(unaff_EBP + -0x14) = DAT_00451a00 ^ *(uint *)(unaff_EBP + 4);
  *(undefined4 *)(unaff_EBP + -0xfa8) = extraout_ECX;
  _eh_vector_constructor_iterator_((void *)(unaff_EBP + -0xedc),0xfc,0xf,FUN_004175df,FUN_0043961a);
  *(undefined4 *)(unaff_EBP + -4) = 0;
  _memset((void *)(unaff_EBP + -0xf1c),0,0x3c);
  *(undefined4 *)(unaff_EBP + -0xedc) = 6;
  *(undefined4 *)(unaff_EBP + -0xec4) = 2;
  FUN_0043ebd0((uint *)(unaff_EBP + -0xe8c),(uint *)"U222A");
  *(undefined4 *)(unaff_EBP + -0xde0) = 6;
  *(undefined4 *)(unaff_EBP + -0xdc8) = 3;
  FUN_0043ebd0((uint *)(unaff_EBP + -0xd90),(uint *)"U999B");
  *(undefined4 *)(unaff_EBP + -0xce4) = 6;
  *(undefined4 *)(unaff_EBP + -0xccc) = 4;
  FUN_0043ebd0((uint *)(unaff_EBP + -0xc94),(uint *)"U999B");
  *(undefined4 *)(unaff_EBP + -0xbe8) = 1;
  *(undefined4 *)(unaff_EBP + -0xbd0) = 2;
  FUN_0043ebd0((uint *)(unaff_EBP + -0xb98),(uint *)"U222A");
  *(undefined4 *)(unaff_EBP + -0xaec) = 1;
  *(undefined4 *)(unaff_EBP + -0xad4) = 3;
  FUN_0043ebd0((uint *)(unaff_EBP + -0xa9c),(uint *)"U999B");
  *(undefined4 *)(unaff_EBP + -0x9f0) = 1;
  *(undefined4 *)(unaff_EBP + -0x9d8) = 4;
  FUN_0043ebd0((uint *)(unaff_EBP + -0x9a0),(uint *)"U999B");
  *(undefined4 *)(unaff_EBP + -0x8f4) = 7;
  *(undefined4 *)(unaff_EBP + -0x8dc) = 2;
  FUN_0043ebd0((uint *)(unaff_EBP + -0x8a4),(uint *)"U222A");
  *(undefined4 *)(unaff_EBP + -0x7f8) = 7;
  *(undefined4 *)(unaff_EBP + -0x7e0) = 3;
  FUN_0043ebd0((uint *)(unaff_EBP + -0x7a8),(uint *)"U999B");
  *(undefined4 *)(unaff_EBP + -0x6fc) = 7;
  *(undefined4 *)(unaff_EBP + -0x6e4) = 4;
  FUN_0043ebd0((uint *)(unaff_EBP + -0x6ac),(uint *)"U999B");
  *(undefined4 *)(unaff_EBP + -0x600) = 2;
  *(undefined4 *)(unaff_EBP + -0x5e8) = 2;
  FUN_0043ebd0((uint *)(unaff_EBP + -0x5b0),(uint *)"U222A");
  *(undefined4 *)(unaff_EBP + -0x504) = 2;
  *(undefined4 *)(unaff_EBP + -0x4ec) = 3;
  FUN_0043ebd0((uint *)(unaff_EBP + -0x4b4),(uint *)"U999B");
  *(undefined4 *)(unaff_EBP + -0x408) = 2;
  *(undefined4 *)(unaff_EBP + -0x3f0) = 4;
  FUN_0043ebd0((uint *)(unaff_EBP + -0x3b8),(uint *)"U666D");
  *(undefined4 *)(unaff_EBP + -0x30c) = 0;
  *(undefined4 *)(unaff_EBP + -0x2f4) = 1;
  FUN_0043ebd0((uint *)(unaff_EBP + -700),(uint *)"U777D");
  *(undefined4 *)(unaff_EBP + -0x210) = 3;
  *(undefined4 *)(unaff_EBP + -0x1f8) = 2;
  FUN_0043ebd0((uint *)(unaff_EBP + -0x1c0),(uint *)"U777D");
  *(undefined4 *)(unaff_EBP + -0x114) = 5;
  *(undefined4 *)(unaff_EBP + -0xfc) = 3;
  FUN_0043ebd0((uint *)(unaff_EBP + -0xc4),(uint *)"U777D");
  pHVar1 = CreateEnhMetaFileA((HDC)0x0,(LPCSTR)0x0,(RECT *)0x0,(LPCSTR)0x0);
  *(HDC *)(unaff_EBP + -0x10) = pHVar1;
  nDenominator = 0x48;
  iVar2 = GetDeviceCaps(*(HDC *)(unaff_EBP + -0x10),0x5a);
  iVar2 = MulDiv(0xc,iVar2,nDenominator);
  *(int *)(unaff_EBP + -0xf1c) = -iVar2;
  *(undefined1 *)(unaff_EBP + -0xf05) = 0;
  *(undefined4 *)(unaff_EBP + -0xf0c) = 100;
  FUN_0043ed39((char *)(unaff_EBP + -0xf00),(byte *)"COURIER NEW");
  pHVar3 = CreateFontIndirectA((LOGFONTA *)(unaff_EBP + -0xf1c));
  *(HFONT *)(unaff_EBP + -0xee0) = pHVar3;
  pvVar4 = SelectObject(*(HDC *)(unaff_EBP + -0x10),*(HGDIOBJ *)(unaff_EBP + -0xee0));
  *(HGDIOBJ *)(unaff_EBP + -0xf90) = pvVar4;
  SetTextColor(*(HDC *)(unaff_EBP + -0x10),0);
  SetBkColor(*(HDC *)(unaff_EBP + -0x10),0xffffff);
  SetBkMode(*(HDC *)(unaff_EBP + -0x10),1);
  *(undefined4 *)(unaff_EBP + -0xf98) = 0;
  *(undefined4 *)(unaff_EBP + -0xf94) = 0;
  *(undefined4 *)(unaff_EBP + -0xf9c) = 0;
  while (*(int *)(unaff_EBP + -0xf9c) < 5) {
    *(undefined4 *)(unaff_EBP + -4000) = 0;
    while (*(int *)(unaff_EBP + -4000) < 3) {
      FUN_00425f03(*(HDC *)(unaff_EBP + -0x10),
                   (int *)(unaff_EBP + -0xedc +
                          (*(int *)(unaff_EBP + -0xf9c) * 3 + *(int *)(unaff_EBP + -4000)) * 0xfc),
                   *(int *)(unaff_EBP + -0xf98),*(int *)(unaff_EBP + -0xf94),1);
      *(int *)(unaff_EBP + -0xf94) = *(int *)(unaff_EBP + -0xf94) + 100;
      *(int *)(unaff_EBP + -4000) = *(int *)(unaff_EBP + -4000) + 1;
    }
    *(undefined4 *)(unaff_EBP + -0xf94) = 0;
    *(int *)(unaff_EBP + -0xf98) = *(int *)(unaff_EBP + -0xf98) + 0x78;
    *(int *)(unaff_EBP + -0xf9c) = *(int *)(unaff_EBP + -0xf9c) + 1;
  }
  pvVar4 = SelectObject(*(HDC *)(unaff_EBP + -0x10),*(HGDIOBJ *)(unaff_EBP + -0xf90));
  DeleteObject(pvVar4);
  pHVar5 = CloseEnhMetaFile(*(HDC *)(unaff_EBP + -0x10));
  *(HENHMETAFILE *)(*(int *)(unaff_EBP + -0xfa8) + 0x16b0) = pHVar5;
  *(undefined4 *)(*(int *)(unaff_EBP + -0xfa8) + 0x1688) =
       *(undefined4 *)(*(int *)(unaff_EBP + -0xfa8) + 0x16b0);
  GetEnhMetaFileHeader
            (*(HENHMETAFILE *)(*(int *)(unaff_EBP + -0xfa8) + 0x1688),0x6c,
             (LPENHMETAHEADER)(unaff_EBP + -0xf8c));
  *(int *)(*(int *)(unaff_EBP + -0xfa8) + 0x16a8) =
       *(int *)(unaff_EBP + -0xf78) - *(int *)(unaff_EBP + -0xf80);
  *(int *)(*(int *)(unaff_EBP + -0xfa8) + 0x16a4) =
       *(int *)(unaff_EBP + -0xf7c) - *(int *)(unaff_EBP + -0xf84);
  *(undefined4 *)(*(int *)(unaff_EBP + -0xfa8) + 0x169c) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0xfa8) + 0x16a0) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0xfa8) + 0x267c) = 1;
  *(undefined4 *)(unaff_EBP + -0xfa4) = 0;
  *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
  _eh_vector_destructor_iterator_((void *)(unaff_EBP + -0xedc),0xfc,0xf,FUN_0043961a);
  ExceptionList = *(void **)(unaff_EBP + -0xc);
  return *(undefined4 *)(unaff_EBP + -0xfa4);
}

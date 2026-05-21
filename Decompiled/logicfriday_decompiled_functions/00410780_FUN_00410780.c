/* 00410780 FUN_00410780 */

undefined4 __thiscall FUN_00410780(void *this,undefined4 param_1,int param_2,uint *param_3)

{
  char cVar1;
  size_t sVar2;
  int iVar3;
  undefined4 uVar4;
  int local_64;
  undefined2 local_60;
  int local_5e;
  undefined2 local_5a;
  undefined2 local_58;
  undefined4 local_56;
  HDC local_50;
  void *local_4c;
  RECT local_48;
  undefined1 local_38 [8];
  int local_30;
  int local_2c;
  HBITMAP local_20;
  HENHMETAFILE local_1c;
  FILE *local_18;
  size_t local_14;
  BITMAPINFO *local_10;
  HDC local_c;
  char *local_8;
  
  local_8 = (char *)0x0;
  local_1c = (HENHMETAFILE)0x0;
  local_c = (HDC)0x0;
  local_20 = (HBITMAP)0x0;
  local_4c = (void *)0x0;
  sVar2 = _strlen((char *)param_3);
  if (sVar2 == 0) {
    return 0x1d0008;
  }
  local_8 = _strrchr((char *)param_3,0x2e);
  if ((local_8 != (char *)0x0) && (iVar3 = __stricmp(local_8 + 1,"emf"), iVar3 == 0)) {
    local_1c = CopyEnhMetaFileA(*(HENHMETAFILE *)(param_2 + 0x1688),(LPCSTR)param_3);
    if (local_1c != (HENHMETAFILE)0x0) {
      DeleteEnhMetaFile(local_1c);
      return 0;
    }
    return 0x1d0007;
  }
  if ((local_8 == (char *)0x0) || (iVar3 = __stricmp(local_8 + 1,"bmp"), iVar3 != 0)) {
    FUN_0043ebe0(param_3,(uint *)&DAT_0044bba0);
  }
  local_48.left = *(LONG *)(param_2 + 0x169c);
  local_48.top = *(LONG *)(param_2 + 0x16a0);
  local_48.right = *(int *)(param_2 + 0x16a4);
  local_48.bottom = *(int *)(param_2 + 0x16a8);
  local_10 = _calloc(1,0x30);
  (local_10->bmiHeader).biSize = 0x28;
  (local_10->bmiHeader).biWidth = local_48.right + 1;
  (local_10->bmiHeader).biHeight = local_48.bottom + 1;
  (local_10->bmiHeader).biPlanes = 1;
  (local_10->bmiHeader).biBitCount = 1;
  (local_10->bmiHeader).biCompression = 0;
  for (local_64 = 0; local_64 < 2; local_64 = local_64 + 1) {
    cVar1 = (char)local_64;
    local_10->bmiColors[local_64].rgbBlue = -('\x01' - cVar1);
    local_10->bmiColors[local_64].rgbGreen = -('\x01' - cVar1);
    local_10->bmiColors[local_64].rgbRed = -('\x01' - cVar1);
    local_10->bmiColors[local_64].rgbReserved = '\0';
  }
  local_20 = CreateDIBSection((HDC)0x0,local_10,0,&local_4c,(HANDLE)0x0,0);
  if (local_20 == (HBITMAP)0x0) {
    uVar4 = 0x1d0007;
  }
  else {
    local_50 = GetDC(*(HWND *)((int)this + 0x26c));
    local_c = CreateCompatibleDC(local_50);
    ReleaseDC(*(HWND *)((int)this + 0x26c),local_50);
    if (local_c == (HDC)0x0) {
      DeleteObject(local_20);
      uVar4 = 0x1d0007;
    }
    else {
      SelectObject(local_c,local_20);
      PlayEnhMetaFile(local_c,*(HENHMETAFILE *)(param_2 + 0x1688),&local_48);
      DeleteDC(local_c);
      GetObjectA(local_20,0x18,local_38);
      local_14 = local_30 * local_2c;
      local_60 = 0x4d42;
      local_5e = local_14 + 0x3e;
      local_56 = 0x3e;
      local_58 = 0;
      local_5a = 0;
      local_18 = (FILE *)FUN_0043e6f2((char *)param_3,"wb");
      if (local_18 == (FILE *)0x0) {
        uVar4 = 0x2b0001;
      }
      else {
        _fwrite(&local_60,0xe,1,local_18);
        _fwrite(local_10,0x30,1,local_18);
        _fwrite(local_4c,1,local_14,local_18);
        _fclose(local_18);
        DeleteObject(local_20);
        _free(local_10);
        uVar4 = 0;
      }
    }
  }
  return uVar4;
}

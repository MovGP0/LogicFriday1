/* 0040cd0c FUN_0040cd0c */

HWND __cdecl FUN_0040cd0c(HWND param_1,HMENU param_2)

{
  HIMAGELIST himl;
  HBITMAP pHVar1;
  HIMAGELIST himl_00;
  void *_Memory;
  int local_c;
  
  himl = ImageList_Create(0x10,0x10,0x19,0xd,0);
  pHVar1 = LoadImageA(DAT_00452914,(LPCSTR)0x168,0,0,0,0);
  ImageList_AddMasked(himl,pHVar1,0xff00ff);
  DeleteObject(pHVar1);
  himl_00 = ImageList_Create(0x10,0x10,0x19,0xd,0);
  pHVar1 = LoadImageA(DAT_00452914,(LPCSTR)0x169,0,0,0,0);
  ImageList_AddMasked(himl_00,pHVar1,0xff00ff);
  DeleteObject(pHVar1);
  DAT_00452a24 = CreateWindowExA(0,"ToolbarWindow32","",0x54000911,0,0,0,0,param_1,param_2,
                                 DAT_00452914,(LPVOID)0x0);
  SendMessageA(DAT_00452a24,0x41e,0x14,0);
  SendMessageA(DAT_00452a24,0x430,0,(LPARAM)himl);
  SendMessageA(DAT_00452a24,0x436,0,(LPARAM)himl_00);
  _Memory = _calloc(DAT_004519a0,0x14);
  for (local_c = 0; local_c < (int)DAT_004519a0; local_c = local_c + 1) {
    *(undefined4 *)((int)_Memory + local_c * 0x14) = *(undefined4 *)(&DAT_004516a0 + local_c * 0x30)
    ;
    *(undefined4 *)((int)_Memory + local_c * 0x14 + 4) =
         *(undefined4 *)(&DAT_004516a4 + local_c * 0x30);
    *(undefined *)((int)_Memory + local_c * 0x14 + 8) = (&DAT_004516a8)[local_c * 0x30];
    *(undefined *)((int)_Memory + local_c * 0x14 + 9) = (&DAT_004516a9)[local_c * 0x30];
    *(undefined4 *)((int)_Memory + local_c * 0x14 + 0x10) =
         *(undefined4 *)(&DAT_004516cc + local_c * 0x30);
  }
  SendMessageA(DAT_00452a24,0x414,DAT_004519a0,(LPARAM)_Memory);
  SendMessageA(DAT_00452a24,0x421,0,0);
  ShowWindow(DAT_00452a24,1);
  _free(_Memory);
  return DAT_00452a24;
}

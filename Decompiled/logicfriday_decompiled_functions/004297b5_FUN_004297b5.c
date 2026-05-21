/* 004297b5 FUN_004297b5 */

void __fastcall FUN_004297b5(int param_1)

{
  void *pvVar1;
  int local_8;
  
  if (*(int *)(param_1 + 0x2318) != 0) {
    DeleteDC(*(HDC *)(param_1 + 0x2318));
    *(undefined4 *)(param_1 + 0x2318) = 0;
  }
  if (*(int *)(param_1 + 0x2314) != 0) {
    DeleteObject(*(HGDIOBJ *)(param_1 + 0x2314));
    *(undefined4 *)(param_1 + 0x2314) = 0;
  }
  if (*(int *)(param_1 + 0x2320) != 0) {
    DeleteObject(*(HGDIOBJ *)(param_1 + 0x2320));
    *(undefined4 *)(param_1 + 0x2320) = 0;
  }
  if (*(int *)(param_1 + 9000) != 0) {
    DeleteObject(*(HGDIOBJ *)(param_1 + 9000));
    *(undefined4 *)(param_1 + 9000) = 0;
  }
  if (*(int *)(param_1 + 0x232c) != 0) {
    DeleteObject(*(HGDIOBJ *)(param_1 + 0x232c));
    *(undefined4 *)(param_1 + 0x232c) = 0;
  }
  if (*(int *)(param_1 + 0x2324) != 0) {
    DeleteObject(*(HGDIOBJ *)(param_1 + 0x2324));
    *(undefined4 *)(param_1 + 0x2324) = 0;
  }
  if (*(int *)(param_1 + 0x2330) != 0) {
    DeleteObject(*(HGDIOBJ *)(param_1 + 0x2330));
    *(undefined4 *)(param_1 + 0x2330) = 0;
  }
  if (*(int *)(param_1 + 0x2334) != 0) {
    DeleteObject(*(HGDIOBJ *)(param_1 + 0x2334));
    *(undefined4 *)(param_1 + 0x2334) = 0;
  }
  for (local_8 = 0; local_8 < *(int *)(param_1 + 0x16c4); local_8 = local_8 + 1) {
    if (*(int *)(*(int *)(param_1 + 0x16cc) + local_8 * 4) != 0) {
      pvVar1 = *(void **)(*(int *)(param_1 + 0x16cc) + local_8 * 4);
      if (pvVar1 != (void *)0x0) {
        FUN_0041d8f2(pvVar1,1);
      }
      *(undefined4 *)(*(int *)(param_1 + 0x16cc) + local_8 * 4) = 0;
    }
  }
  if (1000 < *(uint *)(param_1 + 0x2350)) {
    *(undefined4 *)(param_1 + 0x2350) = 1000;
    pvVar1 = _realloc(*(void **)(param_1 + 0x16cc),*(int *)(param_1 + 0x2350) << 2);
    *(void **)(param_1 + 0x16cc) = pvVar1;
  }
  return;
}

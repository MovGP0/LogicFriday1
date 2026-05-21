/* 0041d5e1 FUN_0041d5e1 */

void FUN_0041d5e1(void)

{
  void *pvVar1;
  undefined4 extraout_ECX;
  int unaff_EBP;
  
  FUN_0043f30c();
  *(undefined4 *)(unaff_EBP + -0x2c) = extraout_ECX;
  *(undefined4 *)(unaff_EBP + -4) = 1;
  *(undefined4 *)(unaff_EBP + -0x10) = 0;
  while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x2c) + 200)) {
    if (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x84 + *(int *)(unaff_EBP + -0x10) * 4) != 0) {
      _free(*(void **)(*(int *)(unaff_EBP + -0x2c) + 0x84 + *(int *)(unaff_EBP + -0x10) * 4));
    }
    if (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x1fc + *(int *)(unaff_EBP + -0x10) * 4) != 0) {
      _free(*(void **)(*(int *)(unaff_EBP + -0x2c) + 0x1fc + *(int *)(unaff_EBP + -0x10) * 4));
    }
    *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
  }
  if (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x1f8) != 0) {
    _free(*(void **)(*(int *)(unaff_EBP + -0x2c) + 0x1f8));
  }
  if (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x268) != 0) {
    _free(*(void **)(*(int *)(unaff_EBP + -0x2c) + 0x268));
  }
  if (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x270) != 0) {
    _free(*(void **)(*(int *)(unaff_EBP + -0x2c) + 0x270));
  }
  if (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x26c) != 0) {
    _free(*(void **)(*(int *)(unaff_EBP + -0x2c) + 0x26c));
  }
  if (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x274) != 0) {
    _free(*(void **)(*(int *)(unaff_EBP + -0x2c) + 0x274));
  }
  if (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x3a4) != 0) {
    *(undefined4 *)(unaff_EBP + -0x18) = *(undefined4 *)(*(int *)(unaff_EBP + -0x2c) + 0x3a4);
    *(undefined4 *)(unaff_EBP + -0x14) = *(undefined4 *)(unaff_EBP + -0x18);
    if (*(int *)(unaff_EBP + -0x14) == 0) {
      *(undefined4 *)(unaff_EBP + -0x30) = 0;
    }
    else {
      pvVar1 = FUN_0041338b(*(void **)(unaff_EBP + -0x14),3);
      *(void **)(unaff_EBP + -0x30) = pvVar1;
    }
  }
  if (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x17e0) != 0) {
    _free(*(void **)(*(int *)(unaff_EBP + -0x2c) + 0x17e0));
  }
  if (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x17dc) != 0) {
    _free(*(void **)(*(int *)(unaff_EBP + -0x2c) + 0x17dc));
  }
  if (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x16b0) != 0) {
    DeleteEnhMetaFile(*(HENHMETAFILE *)(*(int *)(unaff_EBP + -0x2c) + 0x16b0));
  }
  if (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x2678) != 0) {
    *(undefined4 *)(unaff_EBP + -0x10) = 0;
    while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x2c) + 0x2670)) {
      _free(*(void **)(*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x2678) +
                      *(int *)(unaff_EBP + -0x10) * 4));
      *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
    }
    _free(*(void **)(*(int *)(unaff_EBP + -0x2c) + 0x2678));
    *(undefined4 *)(*(int *)(unaff_EBP + -0x2c) + 0x2678) = 0;
  }
  if (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x1668) != 0) {
    _free(*(void **)(*(int *)(unaff_EBP + -0x2c) + 0x1668));
  }
  if (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x2318) != 0) {
    FUN_004297b5(*(int *)(unaff_EBP + -0x2c));
  }
  if (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x16cc) != 0) {
    *(undefined4 *)(unaff_EBP + -0x10) = 0;
    while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x2c) + 0x16c4)) {
      *(undefined4 *)(unaff_EBP + -0x20) =
           *(undefined4 *)
            (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x16cc) + *(int *)(unaff_EBP + -0x10) * 4);
      *(undefined4 *)(unaff_EBP + -0x1c) = *(undefined4 *)(unaff_EBP + -0x20);
      if (*(int *)(unaff_EBP + -0x1c) == 0) {
        *(undefined4 *)(unaff_EBP + -0x34) = 0;
      }
      else {
        pvVar1 = FUN_0041d8f2(*(void **)(unaff_EBP + -0x1c),1);
        *(void **)(unaff_EBP + -0x34) = pvVar1;
      }
      *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
    }
    _free(*(void **)(*(int *)(unaff_EBP + -0x2c) + 0x16cc));
  }
  if (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x16d0) != 0) {
    *(undefined4 *)(unaff_EBP + -0x10) = 0;
    while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x2c) + 0x16c8)) {
      *(undefined4 *)(unaff_EBP + -0x28) =
           *(undefined4 *)
            (*(int *)(*(int *)(unaff_EBP + -0x2c) + 0x16d0) + *(int *)(unaff_EBP + -0x10) * 4);
      *(undefined4 *)(unaff_EBP + -0x24) = *(undefined4 *)(unaff_EBP + -0x28);
      if (*(int *)(unaff_EBP + -0x24) == 0) {
        *(undefined4 *)(unaff_EBP + -0x38) = 0;
      }
      else {
        pvVar1 = FUN_0041d91b(*(void **)(unaff_EBP + -0x24),1);
        *(void **)(unaff_EBP + -0x38) = pvVar1;
      }
      *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
    }
    _free(*(void **)(*(int *)(unaff_EBP + -0x2c) + 0x16d0));
  }
  *(undefined1 *)(unaff_EBP + -4) = 0;
  FUN_0043ab65(*(int *)(unaff_EBP + -0x2c) + 0x24ec);
  *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
  FUN_0043961a();
  ExceptionList = *(void **)(unaff_EBP + -0xc);
  return;
}

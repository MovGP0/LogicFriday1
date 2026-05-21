/* 0041cbd4 FUN_0041cbd4 */

undefined4 FUN_0041cbd4(void)

{
  int iVar1;
  undefined4 uVar2;
  void *pvVar3;
  size_t sVar4;
  HENHMETAFILE pHVar5;
  undefined4 extraout_ECX;
  int iVar6;
  int unaff_EBP;
  undefined4 *puVar7;
  undefined4 *puVar8;
  
  FUN_0043f30c();
  *(undefined4 *)(unaff_EBP + -0x30) = extraout_ECX;
  FUN_004175df(*(int *)(unaff_EBP + -0x30) + 0x23f0);
  *(undefined4 *)(unaff_EBP + -4) = 0;
  FUN_0043ab51((undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x24ec));
  *(undefined1 *)(unaff_EBP + -4) = 1;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x165c) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x165c);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1660) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x1660);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1664) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x1664);
  pvVar3 = _malloc(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x165c) * 0x7fff);
  *(void **)(*(int *)(unaff_EBP + -0x30) + 0x268) = pvVar3;
  pvVar3 = _malloc(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x1660) * 0x7fff);
  *(void **)(*(int *)(unaff_EBP + -0x30) + 0x26c) = pvVar3;
  pvVar3 = _malloc(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x1664) * 0x7fff);
  *(void **)(*(int *)(unaff_EBP + -0x30) + 0x274) = pvVar3;
  FUN_0043ebd0(*(uint **)(*(int *)(unaff_EBP + -0x30) + 0x268),
               *(uint **)(*(int *)(unaff_EBP + 8) + 0x268));
  FUN_0043ebd0(*(uint **)(*(int *)(unaff_EBP + -0x30) + 0x26c),
               *(uint **)(*(int *)(unaff_EBP + 8) + 0x26c));
  FUN_0043ebd0(*(uint **)(*(int *)(unaff_EBP + -0x30) + 0x274),
               *(uint **)(*(int *)(unaff_EBP + 8) + 0x274));
  if (*(int *)(*(int *)(unaff_EBP + 8) + 0x270) != 0) {
    sVar4 = _strlen(*(char **)(*(int *)(unaff_EBP + 8) + 0x270));
    *(size_t *)(unaff_EBP + -0x18) = sVar4;
    pvVar3 = _malloc(*(int *)(unaff_EBP + -0x18) + 1);
    *(void **)(*(int *)(unaff_EBP + -0x30) + 0x270) = pvVar3;
    FUN_0043ebd0(*(uint **)(*(int *)(unaff_EBP + -0x30) + 0x270),
                 *(uint **)(*(int *)(unaff_EBP + 8) + 0x270));
  }
  puVar7 = *(undefined4 **)(unaff_EBP + 8);
  puVar8 = *(undefined4 **)(unaff_EBP + -0x30);
  for (iVar6 = 0x7c; iVar6 != 0; iVar6 = iVar6 + -1) {
    *puVar8 = *puVar7;
    puVar7 = puVar7 + 1;
    puVar8 = puVar8 + 1;
  }
  *(undefined4 *)(unaff_EBP + -0x10) = 0;
  while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x30) + 200)) {
    pvVar3 = _malloc(**(int **)(unaff_EBP + -0x30) << 2);
    *(void **)(*(int *)(unaff_EBP + -0x30) + 0x84 + *(int *)(unaff_EBP + -0x10) * 4) = pvVar3;
    *(undefined4 *)(unaff_EBP + -0x14) = 0;
    while (*(int *)(unaff_EBP + -0x14) < **(int **)(unaff_EBP + -0x30)) {
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x30) + 0x84 + *(int *)(unaff_EBP + -0x10) * 4) +
       *(int *)(unaff_EBP + -0x14) * 4) =
           *(undefined4 *)
            (*(int *)(*(int *)(unaff_EBP + 8) + 0x84 + *(int *)(unaff_EBP + -0x10) * 4) +
            *(int *)(unaff_EBP + -0x14) * 4);
      *(int *)(unaff_EBP + -0x14) = *(int *)(unaff_EBP + -0x14) + 1;
    }
    *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
  }
  puVar7 = (undefined4 *)(*(int *)(unaff_EBP + 8) + 0x1f0);
  puVar8 = (undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1f0);
  for (iVar6 = 0x13; iVar6 != 0; iVar6 = iVar6 + -1) {
    *puVar8 = *puVar7;
    puVar7 = puVar7 + 1;
    puVar8 = puVar8 + 1;
  }
  pvVar3 = _malloc(*(int *)(*(int *)(unaff_EBP + -0x30) + 500) * 0xc);
  *(void **)(*(int *)(unaff_EBP + -0x30) + 0x1f8) = pvVar3;
  *(undefined4 *)(unaff_EBP + -0x10) = 0;
  while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x30) + 500)) {
    puVar7 = (undefined4 *)
             (*(int *)(*(int *)(unaff_EBP + 8) + 0x1f8) + *(int *)(unaff_EBP + -0x10) * 0xc);
    puVar8 = (undefined4 *)
             (*(int *)(*(int *)(unaff_EBP + -0x30) + 0x1f8) + *(int *)(unaff_EBP + -0x10) * 0xc);
    *puVar8 = *puVar7;
    puVar8[1] = puVar7[1];
    puVar8[2] = puVar7[2];
    *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
  }
  *(undefined4 *)(unaff_EBP + -0x10) = 0;
  while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x30) + 200)) {
    pvVar3 = _malloc(*(int *)(*(int *)(unaff_EBP + -0x30) + 500) << 2);
    *(void **)(*(int *)(unaff_EBP + -0x30) + 0x1fc + *(int *)(unaff_EBP + -0x10) * 4) = pvVar3;
    *(undefined4 *)(unaff_EBP + -0x14) = 0;
    while (*(int *)(unaff_EBP + -0x14) < *(int *)(*(int *)(unaff_EBP + -0x30) + 500)) {
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x30) + 0x1fc + *(int *)(unaff_EBP + -0x10) * 4) +
       *(int *)(unaff_EBP + -0x14) * 4) =
           *(undefined4 *)
            (*(int *)(*(int *)(unaff_EBP + 8) + 0x1fc + *(int *)(unaff_EBP + -0x10) * 4) +
            *(int *)(unaff_EBP + -0x14) * 4);
      *(int *)(unaff_EBP + -0x14) = *(int *)(unaff_EBP + -0x14) + 1;
    }
    *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
  }
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x3a4) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1650) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x1650);
  if (*(int *)(*(int *)(unaff_EBP + -0x30) + 0x1650) != 0) {
    *(undefined4 *)(unaff_EBP + -0x1c) = *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1650);
    pvVar3 = operator_new(*(int *)(unaff_EBP + -0x1c) * 0xfc + 4);
    *(void **)(unaff_EBP + -0x24) = pvVar3;
    *(undefined1 *)(unaff_EBP + -4) = 2;
    if (*(int *)(unaff_EBP + -0x24) == 0) {
      *(undefined4 *)(unaff_EBP + -0x34) = 0;
    }
    else {
      **(undefined4 **)(unaff_EBP + -0x24) = *(undefined4 *)(unaff_EBP + -0x1c);
      _eh_vector_constructor_iterator_
                ((void *)(*(int *)(unaff_EBP + -0x24) + 4),0xfc,*(int *)(unaff_EBP + -0x1c),
                 FUN_004175df,FUN_0043961a);
      *(int *)(unaff_EBP + -0x34) = *(int *)(unaff_EBP + -0x24) + 4;
    }
    *(undefined4 *)(unaff_EBP + -0x20) = *(undefined4 *)(unaff_EBP + -0x34);
    *(undefined1 *)(unaff_EBP + -4) = 1;
    *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x3a4) = *(undefined4 *)(unaff_EBP + -0x20);
    *(undefined4 *)(unaff_EBP + -0x10) = 0;
    while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x30) + 0x1650)) {
      puVar7 = (undefined4 *)
               (*(int *)(*(int *)(unaff_EBP + 8) + 0x3a4) + *(int *)(unaff_EBP + -0x10) * 0xfc);
      puVar8 = (undefined4 *)
               (*(int *)(*(int *)(unaff_EBP + -0x30) + 0x3a4) + *(int *)(unaff_EBP + -0x10) * 0xfc);
      for (iVar6 = 0x3f; iVar6 != 0; iVar6 = iVar6 + -1) {
        *puVar8 = *puVar7;
        puVar7 = puVar7 + 1;
        puVar8 = puVar8 + 1;
      }
      *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
    }
  }
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x16c8) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x16c8);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x16c0) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x16c0);
  pvVar3 = _malloc(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16c0) << 2);
  *(void **)(*(int *)(unaff_EBP + -0x30) + 0x16d0) = pvVar3;
  *(undefined4 *)(unaff_EBP + -0x10) = 0;
  while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x30) + 0x16c8)) {
    pvVar3 = operator_new(0x50);
    *(void **)(unaff_EBP + -0x2c) = pvVar3;
    *(undefined1 *)(unaff_EBP + -4) = 3;
    if (*(int *)(unaff_EBP + -0x2c) == 0) {
      *(undefined4 *)(unaff_EBP + -0x38) = 0;
    }
    else {
      puVar7 = FUN_0043ab51(*(undefined4 **)(unaff_EBP + -0x2c));
      *(undefined4 **)(unaff_EBP + -0x38) = puVar7;
    }
    *(undefined4 *)(unaff_EBP + -0x28) = *(undefined4 *)(unaff_EBP + -0x38);
    *(undefined1 *)(unaff_EBP + -4) = 1;
    *(undefined4 *)
     (*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) + *(int *)(unaff_EBP + -0x10) * 4) =
         *(undefined4 *)(unaff_EBP + -0x28);
    puVar7 = *(undefined4 **)
              (*(int *)(*(int *)(unaff_EBP + 8) + 0x16d0) + *(int *)(unaff_EBP + -0x10) * 4);
    puVar8 = *(undefined4 **)
              (*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) + *(int *)(unaff_EBP + -0x10) * 4);
    for (iVar6 = 0x14; iVar6 != 0; iVar6 = iVar6 + -1) {
      *puVar8 = *puVar7;
      puVar7 = puVar7 + 1;
      puVar8 = puVar8 + 1;
    }
    pvVar3 = _malloc(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                                      *(int *)(unaff_EBP + -0x10) * 4) + 0x28) * 0x14);
    *(void **)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                       *(int *)(unaff_EBP + -0x10) * 4) + 0x2c) = pvVar3;
    *(undefined4 *)(unaff_EBP + -0x14) = 0;
    while (*(int *)(unaff_EBP + -0x14) <
           *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                            *(int *)(unaff_EBP + -0x10) * 4) + 0x28)) {
      puVar7 = (undefined4 *)
               (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + 8) + 0x16d0) +
                                 *(int *)(unaff_EBP + -0x10) * 4) + 0x2c) +
               *(int *)(unaff_EBP + -0x14) * 0x14);
      puVar8 = (undefined4 *)
               (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                                 *(int *)(unaff_EBP + -0x10) * 4) + 0x2c) +
               *(int *)(unaff_EBP + -0x14) * 0x14);
      for (iVar6 = 5; iVar6 != 0; iVar6 = iVar6 + -1) {
        *puVar8 = *puVar7;
        puVar7 = puVar7 + 1;
        puVar8 = puVar8 + 1;
      }
      *(int *)(unaff_EBP + -0x14) = *(int *)(unaff_EBP + -0x14) + 1;
    }
    if (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                         *(int *)(unaff_EBP + -0x10) * 4) + 0x30) != 0) {
      pvVar3 = _malloc(*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                                        *(int *)(unaff_EBP + -0x10) * 4) + 0x30) * 0x14);
      *(void **)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                         *(int *)(unaff_EBP + -0x10) * 4) + 0x34) = pvVar3;
      *(undefined4 *)(unaff_EBP + -0x14) = 0;
      while (*(int *)(unaff_EBP + -0x14) <
             *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                              *(int *)(unaff_EBP + -0x10) * 4) + 0x30)) {
        puVar7 = (undefined4 *)
                 (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + 8) + 0x16d0) +
                                   *(int *)(unaff_EBP + -0x10) * 4) + 0x34) +
                 *(int *)(unaff_EBP + -0x14) * 0x14);
        puVar8 = (undefined4 *)
                 (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                                   *(int *)(unaff_EBP + -0x10) * 4) + 0x34) +
                 *(int *)(unaff_EBP + -0x14) * 0x14);
        for (iVar6 = 5; iVar6 != 0; iVar6 = iVar6 + -1) {
          *puVar8 = *puVar7;
          puVar7 = puVar7 + 1;
          puVar8 = puVar8 + 1;
        }
        *(int *)(unaff_EBP + -0x14) = *(int *)(unaff_EBP + -0x14) + 1;
      }
    }
    *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
  }
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x2350) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x2350);
  pvVar3 = _malloc(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x2350) << 2);
  *(void **)(*(int *)(unaff_EBP + -0x30) + 0x16cc) = pvVar3;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x16b0) = 0;
  if (*(int *)(*(int *)(unaff_EBP + 8) + 0x16b0) != 0) {
    pHVar5 = CopyEnhMetaFileA(*(HENHMETAFILE *)(*(int *)(unaff_EBP + 8) + 0x16b0),(LPCSTR)0x0);
    *(HENHMETAFILE *)(*(int *)(unaff_EBP + -0x30) + 0x16b0) = pHVar5;
  }
  puVar7 = (undefined4 *)(*(int *)(unaff_EBP + 8) + 0x1688);
  puVar8 = (undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1688);
  for (iVar6 = 10; iVar6 != 0; iVar6 = iVar6 + -1) {
    *puVar8 = *puVar7;
    puVar7 = puVar7 + 1;
    puVar8 = puVar8 + 1;
  }
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1688) =
       *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x16b0);
  puVar7 = (undefined4 *)(*(int *)(unaff_EBP + 8) + 0x3a8);
  puVar8 = (undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x3a8);
  for (iVar6 = 0x4aa; iVar6 != 0; iVar6 = iVar6 + -1) {
    *puVar8 = *puVar7;
    puVar7 = puVar7 + 1;
    puVar8 = puVar8 + 1;
  }
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1654) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x1654);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1658) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x1658);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x165c) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x165c);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1660) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x1660);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1664) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x1664);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x234c) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x234c);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x267c) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x267c);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x23c) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x23c);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x240) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x240);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x260) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x260);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x244) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x244);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x248) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x248);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x250) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x250);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x254) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x254);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x16b4) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x16b4);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x16b8) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x16b8);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x16bc) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x16bc);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x24c) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x24c);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x264) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x264);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x2308) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x2308);
  iVar6 = *(int *)(unaff_EBP + 8);
  iVar1 = *(int *)(unaff_EBP + -0x30);
  *(undefined4 *)(iVar1 + 0x26f0) = *(undefined4 *)(iVar6 + 0x26f0);
  *(undefined4 *)(iVar1 + 0x26f4) = *(undefined4 *)(iVar6 + 0x26f4);
  *(undefined4 *)(iVar1 + 0x26f8) = *(undefined4 *)(iVar6 + 0x26f8);
  *(undefined4 *)(iVar1 + 0x26fc) = *(undefined4 *)(iVar6 + 0x26fc);
  *(undefined4 *)(unaff_EBP + -0x10) = 0;
  while (*(int *)(unaff_EBP + -0x10) < 0x1a) {
    *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x25ec + *(int *)(unaff_EBP + -0x10) * 4) =
         *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x25ec + *(int *)(unaff_EBP + -0x10) * 4);
    *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
  }
  uVar2 = *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x1674);
  iVar6 = *(int *)(unaff_EBP + -0x30);
  *(undefined4 *)(iVar6 + 0x1670) = *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x1670);
  *(undefined4 *)(iVar6 + 0x1674) = uVar2;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1678) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x1678);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x167c) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x167c);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1680) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x1680);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1684) =
       *(undefined4 *)(*(int *)(unaff_EBP + 8) + 0x1684);
  pvVar3 = _malloc(0x100);
  *(void **)(*(int *)(unaff_EBP + -0x30) + 0x1668) = pvVar3;
  _memset(*(void **)(*(int *)(unaff_EBP + -0x30) + 0x1668),0,0x100);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x17dc) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x17e0) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x16c4) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x270) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x16d4) = 0;
  FUN_0043ebd0((uint *)(*(int *)(unaff_EBP + -0x30) + 0x16fc),(uint *)"Entered:");
  FUN_0043ebd0((uint *)(*(int *)(unaff_EBP + -0x30) + 0x171c),(uint *)"Entered by truthtable:");
  FUN_0043ebd0((uint *)(*(int *)(unaff_EBP + -0x30) + 0x173c),(uint *)"Minimized:");
  FUN_0043ebd0((uint *)(*(int *)(unaff_EBP + -0x30) + 0x179c),(uint *)"Unminimized Product of Sums:"
              );
  FUN_0043ebd0((uint *)(*(int *)(unaff_EBP + -0x30) + 0x175c),(uint *)"Minimized Product of Sums:");
  FUN_004225c6(*(int *)(unaff_EBP + -0x30));
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x2678) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x2318) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x2314) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x2320) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x2324) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 9000) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x232c) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x2330) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x2334) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x16f0) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x16f4) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x26ec) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x23cc) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x23d0) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 600) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x25c) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x2668) = 0;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x266c) = 0;
  *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
  ExceptionList = *(void **)(unaff_EBP + -0xc);
  return *(undefined4 *)(unaff_EBP + -0x30);
}

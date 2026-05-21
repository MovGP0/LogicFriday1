/* 0042fcbc FUN_0042fcbc */

undefined4 FUN_0042fcbc(void)

{
  void *pvVar1;
  undefined4 uVar2;
  int iVar3;
  undefined4 *puVar4;
  undefined4 extraout_ECX;
  int unaff_EBP;
  undefined4 *puVar5;
  
  FUN_0043f30c();
  *(undefined4 *)(unaff_EBP + -0x34) = extraout_ECX;
  *(undefined4 *)(unaff_EBP + -0x18) = 0;
  *(undefined4 *)(unaff_EBP + -0x10) = *(undefined4 *)(*(int *)(unaff_EBP + -0x34) + 0x1650);
  *(undefined4 *)(unaff_EBP + -0x1c) = *(undefined4 *)(*(int *)(unaff_EBP + -0x34) + 0x16c8);
  if (1000 < *(uint *)(*(int *)(unaff_EBP + -0x34) + 0x1650)) {
    pvVar1 = _realloc(*(void **)(unaff_EBP + 0xc),
                      (*(int *)(*(int *)(unaff_EBP + -0x34) + 0x1650) + *(int *)(unaff_EBP + -0x18))
                      * 4 + 4000);
    *(void **)(unaff_EBP + 0xc) = pvVar1;
    if (*(int *)(unaff_EBP + 0xc) == 0) {
      uVar2 = 0x40018;
      goto LAB_00430029;
    }
  }
  if (10000 < *(uint *)(*(int *)(unaff_EBP + -0x34) + 0x16c8)) {
    pvVar1 = _realloc(*(void **)(unaff_EBP + 8),
                      (*(int *)(*(int *)(unaff_EBP + -0x34) + 0x16c8) + *(int *)(unaff_EBP + -0x18))
                      * 4 + 4000);
    *(void **)(unaff_EBP + 8) = pvVar1;
    if (*(int *)(unaff_EBP + 8) == 0) {
      uVar2 = 0x16;
      goto LAB_00430029;
    }
  }
  *(undefined4 *)(unaff_EBP + -0x14) = 0;
  while (*(int *)(unaff_EBP + -0x14) < *(int *)(*(int *)(unaff_EBP + -0x34) + 0x1650)) {
    pvVar1 = operator_new(0xfc);
    *(void **)(unaff_EBP + -0x28) = pvVar1;
    *(undefined4 *)(unaff_EBP + -4) = 0;
    if (*(int *)(unaff_EBP + -0x28) == 0) {
      *(undefined4 *)(unaff_EBP + -0x38) = 0;
    }
    else {
      iVar3 = FUN_004175df(*(int *)(unaff_EBP + -0x28));
      *(int *)(unaff_EBP + -0x38) = iVar3;
    }
    *(undefined4 *)(unaff_EBP + -0x24) = *(undefined4 *)(unaff_EBP + -0x38);
    *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
    *(undefined4 *)(*(int *)(unaff_EBP + 0xc) + *(int *)(unaff_EBP + -0x14) * 4) =
         *(undefined4 *)(unaff_EBP + -0x24);
    puVar4 = (undefined4 *)
             (*(int *)(*(int *)(unaff_EBP + -0x34) + 0x3a4) + *(int *)(unaff_EBP + -0x14) * 0xfc);
    puVar5 = *(undefined4 **)(*(int *)(unaff_EBP + 0xc) + *(int *)(unaff_EBP + -0x14) * 4);
    for (iVar3 = 0x3f; iVar3 != 0; iVar3 = iVar3 + -1) {
      *puVar5 = *puVar4;
      puVar4 = puVar4 + 1;
      puVar5 = puVar5 + 1;
    }
    *(undefined4 *)(*(int *)(*(int *)(unaff_EBP + 0xc) + *(int *)(unaff_EBP + -0x14) * 4) + 0xb4) =
         0;
    *(undefined4 *)(*(int *)(*(int *)(unaff_EBP + 0xc) + *(int *)(unaff_EBP + -0x14) * 4) + 0xb8) =
         0;
    *(undefined4 *)(*(int *)(*(int *)(unaff_EBP + 0xc) + *(int *)(unaff_EBP + -0x14) * 4) + 0xd8) =
         0;
    if (*(int *)(*(int *)(unaff_EBP + -0x14) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x34) + 0x3a4))
        == 8) {
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x34) + 0x3a4) + 0x3c + *(int *)(unaff_EBP + -0x14) * 0xfc) =
           0;
    }
    else {
      *(undefined4 *)(*(int *)(*(int *)(unaff_EBP + 0xc) + *(int *)(unaff_EBP + -0x14) * 4) + 0x3c)
           = 0xfffffffd;
      *(undefined4 *)(*(int *)(*(int *)(unaff_EBP + 0xc) + *(int *)(unaff_EBP + -0x14) * 4) + 0x40)
           = 0xfffffffd;
    }
    *(int *)(unaff_EBP + -0x14) = *(int *)(unaff_EBP + -0x14) + 1;
  }
  *(undefined4 *)(unaff_EBP + -0x14) = 0;
  while (*(int *)(unaff_EBP + -0x14) < *(int *)(*(int *)(unaff_EBP + -0x34) + 0x16c8)) {
    pvVar1 = operator_new(0x50);
    *(void **)(unaff_EBP + -0x30) = pvVar1;
    *(undefined4 *)(unaff_EBP + -4) = 1;
    if (*(int *)(unaff_EBP + -0x30) == 0) {
      *(undefined4 *)(unaff_EBP + -0x3c) = 0;
    }
    else {
      puVar4 = FUN_0043ab51(*(undefined4 **)(unaff_EBP + -0x30));
      *(undefined4 **)(unaff_EBP + -0x3c) = puVar4;
    }
    *(undefined4 *)(unaff_EBP + -0x2c) = *(undefined4 *)(unaff_EBP + -0x3c);
    *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
    *(undefined4 *)(*(int *)(unaff_EBP + 8) + *(int *)(unaff_EBP + -0x14) * 4) =
         *(undefined4 *)(unaff_EBP + -0x2c);
    *(undefined4 *)(*(int *)(*(int *)(unaff_EBP + 8) + *(int *)(unaff_EBP + -0x14) * 4) + 0x4c) =
         *(undefined4 *)(unaff_EBP + -0x14);
    puVar4 = *(undefined4 **)
              (*(int *)(*(int *)(unaff_EBP + -0x34) + 0x16d0) + *(int *)(unaff_EBP + -0x14) * 4);
    puVar5 = *(undefined4 **)(*(int *)(unaff_EBP + 8) + *(int *)(unaff_EBP + -0x14) * 4);
    for (iVar3 = 0x14; iVar3 != 0; iVar3 = iVar3 + -1) {
      *puVar5 = *puVar4;
      puVar4 = puVar4 + 1;
      puVar5 = puVar5 + 1;
    }
    pvVar1 = _malloc(*(int *)(*(int *)(*(int *)(unaff_EBP + 8) + *(int *)(unaff_EBP + -0x14) * 4) +
                             0x28) * 0x14);
    *(void **)(*(int *)(*(int *)(unaff_EBP + 8) + *(int *)(unaff_EBP + -0x14) * 4) + 0x2c) = pvVar1;
    *(undefined4 *)(unaff_EBP + -0x20) = 0;
    while (*(int *)(unaff_EBP + -0x20) <
           *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x34) + 0x16d0) +
                            *(int *)(unaff_EBP + -0x14) * 4) + 0x28)) {
      puVar4 = (undefined4 *)
               (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x34) + 0x16d0) +
                                 *(int *)(unaff_EBP + -0x14) * 4) + 0x2c) +
               *(int *)(unaff_EBP + -0x20) * 0x14);
      puVar5 = (undefined4 *)
               (*(int *)(*(int *)(*(int *)(unaff_EBP + 8) + *(int *)(unaff_EBP + -0x14) * 4) + 0x2c)
               + *(int *)(unaff_EBP + -0x20) * 0x14);
      for (iVar3 = 5; iVar3 != 0; iVar3 = iVar3 + -1) {
        *puVar5 = *puVar4;
        puVar4 = puVar4 + 1;
        puVar5 = puVar5 + 1;
      }
      *(int *)(unaff_EBP + -0x20) = *(int *)(unaff_EBP + -0x20) + 1;
    }
    pvVar1 = _malloc(*(int *)(*(int *)(*(int *)(unaff_EBP + 8) + *(int *)(unaff_EBP + -0x14) * 4) +
                             0x30) * 0x14);
    *(void **)(*(int *)(*(int *)(unaff_EBP + 8) + *(int *)(unaff_EBP + -0x14) * 4) + 0x34) = pvVar1;
    *(undefined4 *)(unaff_EBP + -0x20) = 0;
    while (*(int *)(unaff_EBP + -0x20) <
           *(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x34) + 0x16d0) +
                            *(int *)(unaff_EBP + -0x14) * 4) + 0x30)) {
      puVar4 = (undefined4 *)
               (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x34) + 0x16d0) +
                                 *(int *)(unaff_EBP + -0x14) * 4) + 0x34) +
               *(int *)(unaff_EBP + -0x20) * 0x14);
      puVar5 = (undefined4 *)
               (*(int *)(*(int *)(*(int *)(unaff_EBP + 8) + *(int *)(unaff_EBP + -0x14) * 4) + 0x34)
               + *(int *)(unaff_EBP + -0x20) * 0x14);
      for (iVar3 = 5; iVar3 != 0; iVar3 = iVar3 + -1) {
        *puVar5 = *puVar4;
        puVar4 = puVar4 + 1;
        puVar5 = puVar5 + 1;
      }
      *(int *)(unaff_EBP + -0x20) = *(int *)(unaff_EBP + -0x20) + 1;
    }
    *(int *)(unaff_EBP + -0x14) = *(int *)(unaff_EBP + -0x14) + 1;
  }
  **(undefined4 **)(unaff_EBP + 0x10) = *(undefined4 *)(*(int *)(unaff_EBP + -0x34) + 0x1650);
  **(undefined4 **)(unaff_EBP + 0x14) = *(undefined4 *)(*(int *)(unaff_EBP + -0x34) + 0x16c8);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x34) + 0x1650) = *(undefined4 *)(unaff_EBP + -0x10);
  *(undefined4 *)(*(int *)(unaff_EBP + -0x34) + 0x16c8) = *(undefined4 *)(unaff_EBP + -0x1c);
  uVar2 = 0;
LAB_00430029:
  ExceptionList = *(void **)(unaff_EBP + -0xc);
  return uVar2;
}

/* 00435399 FUN_00435399 */

undefined4 FUN_00435399(void)

{
  void *pvVar1;
  undefined4 extraout_ECX;
  int iVar2;
  int unaff_EBP;
  undefined4 *puVar3;
  undefined4 *puVar4;
  
  FUN_0043f30c();
  *(undefined4 *)(unaff_EBP + -0x30) = extraout_ECX;
  if (*(int *)(*(int *)(unaff_EBP + -0x30) + 0x3a4) != 0) {
    *(undefined4 *)(unaff_EBP + -0x20) = *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x3a4);
    *(undefined4 *)(unaff_EBP + -0x1c) = *(undefined4 *)(unaff_EBP + -0x20);
    if (*(int *)(unaff_EBP + -0x1c) == 0) {
      *(undefined4 *)(unaff_EBP + -0x34) = 0;
    }
    else {
      pvVar1 = FUN_0041338b(*(void **)(unaff_EBP + -0x1c),3);
      *(void **)(unaff_EBP + -0x34) = pvVar1;
    }
  }
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1654) =
       *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0xc4);
  *(int *)(*(int *)(unaff_EBP + -0x30) + 0x1658) =
       *(int *)(*(int *)(unaff_EBP + -0x30) + 0x1654) + *(int *)(unaff_EBP + 8);
  *(int *)(*(int *)(unaff_EBP + -0x30) + 0x1650) =
       *(int *)(unaff_EBP + 8) + *(int *)(*(int *)(unaff_EBP + -0x30) + 0xc4) +
       *(int *)(*(int *)(unaff_EBP + -0x30) + 200);
  *(undefined4 *)(unaff_EBP + -0x24) = *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1650);
  pvVar1 = operator_new(*(int *)(unaff_EBP + -0x24) * 0xfc + 4);
  *(void **)(unaff_EBP + -0x2c) = pvVar1;
  *(undefined4 *)(unaff_EBP + -4) = 0;
  if (*(int *)(unaff_EBP + -0x2c) == 0) {
    *(undefined4 *)(unaff_EBP + -0x38) = 0;
  }
  else {
    **(undefined4 **)(unaff_EBP + -0x2c) = *(undefined4 *)(unaff_EBP + -0x24);
    _eh_vector_constructor_iterator_
              ((void *)(*(int *)(unaff_EBP + -0x2c) + 4),0xfc,*(int *)(unaff_EBP + -0x24),
               FUN_004175df,FUN_0043961a);
    *(int *)(unaff_EBP + -0x38) = *(int *)(unaff_EBP + -0x2c) + 4;
  }
  *(undefined4 *)(unaff_EBP + -0x28) = *(undefined4 *)(unaff_EBP + -0x38);
  *(undefined4 *)(unaff_EBP + -4) = 0xffffffff;
  *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x3a4) = *(undefined4 *)(unaff_EBP + -0x28);
  *(undefined4 *)(unaff_EBP + -0x18) = 0;
  *(undefined4 *)(unaff_EBP + -0x10) = 0;
  while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x30) + 0x16c4)) {
    if ((**(int **)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) + *(int *)(unaff_EBP + -0x10) * 4
                   ) == 8) &&
       (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) +
                         *(int *)(unaff_EBP + -0x10) * 4) + 0x48) == 0)) {
      puVar3 = *(undefined4 **)
                (*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) + *(int *)(unaff_EBP + -0x10) * 4);
      puVar4 = (undefined4 *)
               (*(int *)(*(int *)(unaff_EBP + -0x30) + 0x3a4) + *(int *)(unaff_EBP + -0x18) * 0xfc);
      for (iVar2 = 0x3f; iVar2 != 0; iVar2 = iVar2 + -1) {
        *puVar4 = *puVar3;
        puVar3 = puVar3 + 1;
        puVar4 = puVar4 + 1;
      }
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x30) + 0x3a4) + 0x40 + *(int *)(unaff_EBP + -0x18) * 0xfc) =
           *(undefined4 *)(unaff_EBP + -0x18);
      *(undefined4 *)
       (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) + *(int *)(unaff_EBP + -0x10) * 4) +
       0x4c) = *(undefined4 *)(unaff_EBP + -0x18);
      *(int *)(unaff_EBP + -0x18) = *(int *)(unaff_EBP + -0x18) + 1;
    }
    *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
  }
  *(undefined4 *)(unaff_EBP + -0x18) = *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1654);
  *(undefined4 *)(unaff_EBP + -0x10) = 0;
  while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x30) + 0x16c4)) {
    if (((*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) +
                           *(int *)(unaff_EBP + -0x10) * 4) + 0x48) == 0) &&
        (**(int **)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) + *(int *)(unaff_EBP + -0x10) * 4
                   ) != 8)) &&
       (**(int **)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) + *(int *)(unaff_EBP + -0x10) * 4)
        != 9)) {
      puVar3 = *(undefined4 **)
                (*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) + *(int *)(unaff_EBP + -0x10) * 4);
      puVar4 = (undefined4 *)
               (*(int *)(*(int *)(unaff_EBP + -0x30) + 0x3a4) + *(int *)(unaff_EBP + -0x18) * 0xfc);
      for (iVar2 = 0x3f; iVar2 != 0; iVar2 = iVar2 + -1) {
        *puVar4 = *puVar3;
        puVar3 = puVar3 + 1;
        puVar4 = puVar4 + 1;
      }
      FUN_0041770d((int *)(*(int *)(unaff_EBP + -0x18) * 0xfc +
                          *(int *)(*(int *)(unaff_EBP + -0x30) + 0x3a4)));
      *(undefined4 *)
       (*(int *)(*(int *)(unaff_EBP + -0x30) + 0x3a4) + 0x44 + *(int *)(unaff_EBP + -0x18) * 0xfc) =
           *(undefined4 *)(unaff_EBP + -0x18);
      *(undefined4 *)
       (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) + *(int *)(unaff_EBP + -0x10) * 4) +
       0x4c) = *(undefined4 *)(unaff_EBP + -0x18);
      *(int *)(unaff_EBP + -0x18) = *(int *)(unaff_EBP + -0x18) + 1;
    }
    *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
  }
  *(undefined4 *)(unaff_EBP + -0x18) = *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1658);
  *(undefined4 *)(unaff_EBP + -0x10) = 0;
  while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x30) + 0x16c4)) {
    if ((**(int **)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) + *(int *)(unaff_EBP + -0x10) * 4
                   ) == 9) &&
       (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) +
                         *(int *)(unaff_EBP + -0x10) * 4) + 0x48) == 0)) {
      puVar3 = *(undefined4 **)
                (*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) + *(int *)(unaff_EBP + -0x10) * 4);
      puVar4 = (undefined4 *)
               (*(int *)(*(int *)(unaff_EBP + -0x30) + 0x3a4) + *(int *)(unaff_EBP + -0x18) * 0xfc);
      for (iVar2 = 0x3f; iVar2 != 0; iVar2 = iVar2 + -1) {
        *puVar4 = *puVar3;
        puVar3 = puVar3 + 1;
        puVar4 = puVar4 + 1;
      }
      *(undefined4 *)
       (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) + *(int *)(unaff_EBP + -0x10) * 4) +
       0x4c) = *(undefined4 *)(unaff_EBP + -0x18);
      *(int *)(unaff_EBP + -0x18) = *(int *)(unaff_EBP + -0x18) + 1;
    }
    *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
  }
  *(undefined4 *)(unaff_EBP + -0x10) = *(undefined4 *)(*(int *)(unaff_EBP + -0x30) + 0x1654);
  while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x30) + 0x1650)) {
    *(undefined4 *)(unaff_EBP + -0x18) = 0;
    while (*(int *)(unaff_EBP + -0x18) <
           *(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x3a4) + 0x18 +
                   *(int *)(unaff_EBP + -0x10) * 0xfc)) {
      *(undefined4 *)(unaff_EBP + -0x14) =
           *(undefined4 *)
            (*(int *)(unaff_EBP + -0x10) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x30) + 0x3a4) +
             0x1c + *(int *)(unaff_EBP + -0x18) * 4);
      *(undefined4 *)
       (*(int *)(unaff_EBP + -0x10) * 0xfc + *(int *)(*(int *)(unaff_EBP + -0x30) + 0x3a4) + 0x1c +
       *(int *)(unaff_EBP + -0x18) * 4) =
           *(undefined4 *)
            (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) +
                     *(int *)(unaff_EBP + -0x14) * 4) + 0x4c);
      *(int *)(unaff_EBP + -0x18) = *(int *)(unaff_EBP + -0x18) + 1;
    }
    *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
  }
  *(undefined4 *)(unaff_EBP + -0x10) = 0;
  while (*(int *)(unaff_EBP + -0x10) < *(int *)(*(int *)(unaff_EBP + -0x30) + 0x16c8)) {
    if (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                         *(int *)(unaff_EBP + -0x10) * 4) + 0x40) == 0) {
      if ((**(int **)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                     *(int *)(unaff_EBP + -0x10) * 4) == 0) ||
         (**(int **)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                    *(int *)(unaff_EBP + -0x10) * 4) == 1)) {
        *(undefined4 *)(unaff_EBP + -0x14) =
             *(undefined4 *)
              (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                       *(int *)(unaff_EBP + -0x10) * 4) + 4);
        *(undefined4 *)
         (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) + *(int *)(unaff_EBP + -0x10) * 4)
         + 4) = *(undefined4 *)
                 (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) +
                          *(int *)(unaff_EBP + -0x14) * 4) + 0x4c);
      }
      if ((*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                            *(int *)(unaff_EBP + -0x10) * 4) + 0x14) == 0) ||
         (*(int *)(*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                           *(int *)(unaff_EBP + -0x10) * 4) + 0x14) == 1)) {
        *(undefined4 *)(unaff_EBP + -0x14) =
             *(undefined4 *)
              (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                       *(int *)(unaff_EBP + -0x10) * 4) + 0x18);
        *(undefined4 *)
         (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) + *(int *)(unaff_EBP + -0x10) * 4)
         + 0x18) = *(undefined4 *)
                    (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) +
                             *(int *)(unaff_EBP + -0x14) * 4) + 0x4c);
      }
      *(undefined4 *)(unaff_EBP + -0x14) =
           *(undefined4 *)
            (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) +
                     *(int *)(unaff_EBP + -0x10) * 4) + 0x38);
      *(undefined4 *)
       (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16d0) + *(int *)(unaff_EBP + -0x10) * 4) +
       0x38) = *(undefined4 *)
                (*(int *)(*(int *)(*(int *)(unaff_EBP + -0x30) + 0x16cc) +
                         *(int *)(unaff_EBP + -0x14) * 4) + 0x4c);
    }
    *(int *)(unaff_EBP + -0x10) = *(int *)(unaff_EBP + -0x10) + 1;
  }
  ExceptionList = *(void **)(unaff_EBP + -0xc);
  return 0;
}

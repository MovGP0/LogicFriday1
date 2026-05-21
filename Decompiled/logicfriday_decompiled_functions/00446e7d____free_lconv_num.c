/* 00446e7d ___free_lconv_num */

/* Library Function - Single Match
    ___free_lconv_num
   
   Library: Visual Studio 2003 Release */

void __cdecl ___free_lconv_num(undefined4 *param_1)

{
  undefined *puVar1;
  
  if (param_1 != (undefined4 *)0x0) {
    puVar1 = (undefined *)*param_1;
    if ((puVar1 != *(undefined **)PTR_PTR_00452414) && (puVar1 != PTR_DAT_004523e4)) {
      _free(puVar1);
    }
    puVar1 = (undefined *)param_1[1];
    if ((puVar1 != *(undefined **)(PTR_PTR_00452414 + 4)) && (puVar1 != PTR_DAT_004523e8)) {
      _free(puVar1);
    }
    puVar1 = (undefined *)param_1[2];
    if ((puVar1 != *(undefined **)(PTR_PTR_00452414 + 8)) && (puVar1 != PTR_DAT_004523ec)) {
      _free(puVar1);
    }
  }
  return;
}

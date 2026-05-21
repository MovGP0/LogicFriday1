/* 004407ab FUN_004407ab */

void __cdecl FUN_004407ab(uint param_1)

{
  _ptiddata p_Var1;
  pthreadlocinfo ptVar2;
  void *extraout_ECX;
  void *extraout_ECX_00;
  void *this;
  
  p_Var1 = __getptd();
  ptVar2 = (pthreadlocinfo)p_Var1->_tfpecode;
  this = extraout_ECX;
  if (ptVar2 != (pthreadlocinfo)PTR_DAT_00451fcc) {
    ptVar2 = ___updatetlocinfo();
    this = extraout_ECX_00;
  }
  FUN_004406e3(this,(uint)ptVar2,param_1);
  return;
}

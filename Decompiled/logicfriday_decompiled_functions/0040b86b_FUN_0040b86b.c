/* 0040b86b FUN_0040b86b */

void * __thiscall FUN_0040b86b(void *this,uint param_1)

{
  FUN_0041d5e1();
  if ((param_1 & 1) != 0) {
    _free(this);
  }
  return this;
}
